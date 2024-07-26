use std::any::Any;
use std::error::Error;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use grep::matcher::{LineTerminator, Matcher};
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use rustc_hash::FxHashMap;
use sanedit_buffer::ReadOnlyPieceTree;
use sanedit_utils::sorted_vec::SortedVec;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::actions::jobs::{OptionProvider, CHANNEL_SIZE};
use crate::actions::locations;
use crate::common::matcher::MatchOption;
use crate::common::range::RangeUtils;
use crate::common::search::PTSearcher;
use crate::editor::windows::{Group, Item, SearchDirection, SearchKind};
use crate::{
    editor::{job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

use super::FileOptionProvider;

#[derive(Clone)]
pub(crate) struct Grep {
    client_id: ClientId,
    pattern: String,
    file_opt_provider: FileOptionProvider,
    buffers: Arc<FxHashMap<PathBuf, ReadOnlyPieceTree>>,
}

impl Grep {
    pub fn new(
        pattern: &str,
        path: &Path,
        ignore: &[String],
        buffers: FxHashMap<PathBuf, ReadOnlyPieceTree>,
        id: ClientId,
    ) -> Grep {
        let fprovider = FileOptionProvider::new(path, ignore);

        Grep {
            client_id: id,
            pattern: pattern.into(),
            file_opt_provider: fprovider,
            buffers: Arc::new(buffers),
        }
    }

    async fn grep(
        mut orecv: Receiver<MatchOption>,
        pattern: &str,
        msend: Sender<GrepResult>,
        buffers: Arc<FxHashMap<PathBuf, ReadOnlyPieceTree>>,
    ) {
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_terminator(LineTerminator::byte(b'\n'))
            .line_number(true)
            .multi_line(false)
            .build();

        let matcher = RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .case_insensitive(false)
            .case_smart(false)
            .word(false)
            .build(&pattern)
            .expect("Cannot build RegexMatcher");

        let ptsearcher = Arc::new(
            PTSearcher::new(&pattern, SearchDirection::Forward, SearchKind::Regex)
                .expect("Cannot build PTSearcher"),
        );

        while let Some(opt) = orecv.recv().await {
            let ptsearcher = ptsearcher.clone();
            let mut searcher = searcher.clone();
            let matcher = matcher.clone();
            let msend = msend.clone();
            let bufs = buffers.clone();

            rayon::spawn(move || {
                let Some(path) = opt.path() else {
                    return;
                };

                if let Some(buf) = bufs.get(&path) {
                    // Grep buffer if it exists
                    Self::grep_buffer(path, buf, &ptsearcher, msend);
                } else {
                    // Otherwise use ripgrep
                    let rsend = ResultSender {
                        matcher: &matcher,
                        sender: msend,
                        path: &path,
                        matches: SortedVec::new(),
                    };
                    let _ = searcher.search_path(&matcher, &path, rsend);
                }
            });
        }
    }

    fn grep_buffer(
        path: PathBuf,
        ropt: &ReadOnlyPieceTree,
        searcher: &PTSearcher,
        msend: Sender<GrepResult>,
    ) {
        let slice = ropt.slice(..);
        let mut line_ranges = vec![];
        let mut matches = SortedVec::new();
        let mut lines = slice.lines();
        let mut linen = 1;
        let mut line = lines.next().unwrap();

        for mat in searcher.find_iter(&slice) {
            if line.range().includes(&mat.range()) {
                // Offsets to line start
                let mut range = mat.range();
                range.start -= line.start();
                range.end -= line.start();
                line_ranges.push(range);
                continue;
            }

            if !line_ranges.is_empty() {
                let text = String::from(&line);
                let mat = GrepMatch {
                    line: Some(linen),
                    text,
                    matches: std::mem::take(&mut line_ranges),
                    absolute_offset: Some(line.start() as u64),
                };
                matches.push(mat);
            }

            while !line.range().includes(&mat.range()) {
                line = lines.next().unwrap();
                linen += 1;
            }
            line_ranges.push(mat.range());
        }

        if !line_ranges.is_empty() {
            let text = String::from(&line);
            let mat = GrepMatch {
                line: Some(linen),
                text: text.trim_end().into(),
                matches: std::mem::take(&mut line_ranges),
                absolute_offset: Some(line.start() as u64),
            };
            matches.push(mat);
        }

        if !matches.is_empty() {
            let result = GrepResult { path, matches };
            let _ = msend.blocking_send(result);
        }
    }

    async fn send_results(mut recv: Receiver<GrepResult>, mut ctx: JobContext) {
        ctx.send(Start);

        while let Some(msg) = recv.recv().await {
            ctx.send(msg);
        }
    }
}

impl Job for Grep {
    fn run(&self, ctx: JobContext) -> JobResult {
        let fopts = self.file_opt_provider.clone();
        let pattern = self.pattern.clone();
        let bufs = self.buffers.clone();

        let fut = async move {
            // Options channel
            let (osend, orecv) = channel::<MatchOption>(CHANNEL_SIZE);
            // Results channel
            let (msend, mrecv) = channel::<GrepResult>(CHANNEL_SIZE);

            tokio::join!(
                fopts.provide(osend, ctx.kill.clone()),
                Self::grep(orecv, &pattern, msend, bufs),
                Self::send_results(mrecv, ctx),
            );

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for Grep {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, mut msg: Box<dyn Any>) {
        if let Some(_msg) = msg.downcast_mut::<Start>() {
            locations::clear.execute(editor, self.client_id);
            locations::show.execute(editor, self.client_id);
            return;
        }

        if let Ok(msg) = msg.downcast::<GrepResult>() {
            let items = msg.matches.into_iter().map(Item::from).collect();
            let group = Group {
                expanded: false,
                path: msg.path,
                items,
            };
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            win.locations.push(group);
        }
    }

    fn on_success(&self, editor: &mut Editor) {
        let (win, _buf) = editor.win_buf_mut(self.client_id);
    }

    fn on_failure(&self, editor: &mut Editor, reason: &str) {
        let (win, _buf) = editor.win_buf_mut(self.client_id);
        win.locations.clear();
    }
}

struct Start;

#[derive(Debug, PartialEq, Eq)]
struct GrepMatch {
    line: Option<u64>,
    text: String,
    matches: Vec<Range<usize>>,
    absolute_offset: Option<u64>,
}

impl PartialOrd for GrepMatch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (other.line, &other.text).partial_cmp(&(self.line, &self.text))
    }
}

impl Ord for GrepMatch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.line, &other.text).cmp(&(self.line, &self.text))
    }
}

impl From<GrepMatch> for Item {
    fn from(gmat: GrepMatch) -> Self {
        Item {
            name: gmat.text,
            line: gmat.line,
            column: None,
            highlights: gmat.matches,
            absolute_offset: gmat.absolute_offset,
        }
    }
}

#[derive(Debug)]
struct GrepResult {
    path: PathBuf,
    matches: SortedVec<GrepMatch>,
}

#[derive(Debug)]
struct ResultSender<'a> {
    matcher: &'a RegexMatcher,
    sender: Sender<GrepResult>,
    path: &'a Path,
    matches: SortedVec<GrepMatch>,
}

impl<'a> Sink for ResultSender<'a> {
    type Error = Box<dyn Error>;

    fn matched(&mut self, searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let Ok(text) = std::str::from_utf8(mat.bytes()) else {
            return Ok(true);
        };
        let text = text.trim_end();

        let mut matches = vec![];
        self.matcher
            .find_iter(mat.bytes(), |m| {
                matches.push(m.start()..m.end());
                true
            })
            .ok();

        if !matches.is_empty() {
            let gmat = GrepMatch {
                text: text.to_string(),
                matches,
                line: mat.line_number(),
                absolute_offset: mat.absolute_byte_offset().into(),
            };

            self.matches.push(gmat);
        }

        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &Searcher,
        _: &grep::searcher::SinkFinish,
    ) -> Result<(), Self::Error> {
        let matches = std::mem::take(&mut self.matches);
        if !matches.is_empty() {
            let _ = self.sender.blocking_send(GrepResult {
                path: self.path.to_path_buf(),
                matches,
            });
        }

        Ok(())
    }
}
