use std::any::Any;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use grep::matcher::{LineTerminator, Matcher};
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use rustc_hash::FxHashMap;
use sanedit_buffer::utf8::EndOfLine;
use sanedit_buffer::{PieceTree, PieceTreeSlice, PieceTreeView};
use sanedit_core::{BufferRange, Group, Item, Searcher, Range, SearchDirection, SearchKind};
use sanedit_utils::either::Either;
use sanedit_utils::sorted_vec::SortedVec;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::actions::jobs::{OptionProvider, CHANNEL_SIZE};
use crate::actions::locations;
use crate::common::matcher::Choice;
use crate::editor::{job_broker::KeepInTouch, Editor};
use sanedit_server::{ClientId, Job, JobContext, JobResult};

use super::FileOptionProvider;

#[derive(Clone)]
pub(crate) struct Grep {
    client_id: ClientId,
    pattern: String,
    file_opt_provider: FileOptionProvider,
    buffers: Arc<FxHashMap<PathBuf, PieceTreeView>>,
}

impl Grep {
    pub fn new(
        pattern: &str,
        path: &Path,
        ignore: &[String],
        buffers: FxHashMap<PathBuf, PieceTreeView>,
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
        mut orecv: Receiver<Arc<Choice>>,
        pattern: &str,
        msend: Sender<GrepResult>,
        buffers: Arc<FxHashMap<PathBuf, PieceTreeView>>,
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
            .build(pattern)
            .expect("Cannot build RegexMatcher");

        let ptsearcher = Arc::new(
            Searcher::new(pattern, SearchDirection::Forward, SearchKind::Regex)
                .expect("Cannot build PTSearcher"),
        );

        while let Some(opt) = orecv.recv().await {
            let ptsearcher = ptsearcher.clone();
            let mut searcher = searcher.clone();
            let matcher = matcher.clone();
            let msend = msend.clone();
            let bufs = buffers.clone();

            rayon::spawn(move || {
                let path = match opt.as_ref() {
                    Choice::Path { path, .. } => path,
                    _ => return,
                };

                if let Some(buf) = bufs.get(path) {
                    // Grep buffer if it exists
                    Self::grep_buffer(path.clone(), buf, &ptsearcher, msend);
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
        ropt: &PieceTreeView,
        searcher: &Searcher,
        msend: Sender<GrepResult>,
    ) {
        let slice = ropt.slice(..);
        let mut matches = SortedVec::new();

        // Track lines while iterating
        let mut lines = slice.lines();
        let mut linen = 1;
        let mut line = lines.next().unwrap();
        let mut line_found_matches = vec![];

        for mat in searcher.find_iter(&slice) {
            let line_range: BufferRange = line.range().into();
            // Found a match at current line, add it and continue search
            if line_range.includes(&mat.range()) {
                // Offsets to line start
                let Range { mut start, mut end } = mat.range();
                start -= line.start();
                end -= line.start();

                line_found_matches.push((start as usize..end as usize).into());
                continue;
            }

            // Match is not at current line

            // Add grep match from previous line if it had matches
            if !line_found_matches.is_empty() {
                let mat = prepare_grep_match(
                    Either::Right(&line),
                    Some(linen),
                    line.start(),
                    std::mem::take(&mut line_found_matches),
                );
                matches.push(mat);
            }

            // Iterate to the line the match was found at
            while !line_range.includes(&mat.range()) {
                match lines.next() {
                    Some(l) => {
                        line = l;
                        linen += 1;
                    }
                    None => break,
                }
            }

            // Add match to line_ranges
            let Range { mut start, mut end } = mat.range();
            start -= line.start();
            end -= line.start();
            line_found_matches.push((start as usize..end as usize).into());
        }

        if !line_found_matches.is_empty() {
            let text = String::from(&line);
            let mat = GrepMatch {
                line: Some(linen),
                text: text.trim_end().into(),
                matches: std::mem::take(&mut line_found_matches),
                absolute_offset: Some(line.start()),
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
            let (osend, orecv) = channel::<Arc<Choice>>(CHANNEL_SIZE);
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
            locations::clear_locations.execute(editor, self.client_id);
            locations::show_locations.execute(editor, self.client_id);
            return;
        }

        if let Ok(msg) = msg.downcast::<GrepResult>() {
            let items: Vec<Item> = msg.matches.into_iter().map(Item::from).collect();
            let mut group = Group::new(&msg.path);
            items.into_iter().for_each(|i| group.push(i));
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            win.locations.push(group);
        }
    }

    fn on_success(&self, editor: &mut Editor) {
        let (_win, _buf) = editor.win_buf_mut(self.client_id);
    }

    fn on_failure(&self, editor: &mut Editor, reason: &str) {
        log::error!("Grep error: {reason}");
        let (win, _buf) = editor.win_buf_mut(self.client_id);
        win.locations.clear();
    }
}

struct Start;

#[derive(Debug, PartialEq, Eq)]
struct GrepMatch {
    line: Option<u64>,
    text: String,

    /// Matches found in text
    matches: Vec<Range<usize>>,
    absolute_offset: Option<u64>,
}

impl PartialOrd for GrepMatch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GrepMatch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (other.line, &other.text).cmp(&(self.line, &self.text))
    }
}

impl From<GrepMatch> for Item {
    fn from(gmat: GrepMatch) -> Self {
        Item::new(&gmat.text, gmat.line, gmat.absolute_offset, gmat.matches)
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

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let mut matches = vec![];
        self.matcher
            .find_iter(mat.bytes(), |m| {
                matches.push((m.start()..m.end()).into());
                true
            })
            .ok();

        if !matches.is_empty() {
            let gmat = prepare_grep_match(
                Either::Left(mat.bytes()),
                mat.line_number(),
                mat.absolute_byte_offset(),
                matches,
            );
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

/// Shorten long grep lines to MAX_BEFORE_TRUNC characters.
/// Also move to the match if it is far into the match
fn prepare_grep_match(
    text: Either<&[u8], &PieceTreeSlice>,
    line: Option<u64>,
    mut offset: u64,
    mut matches: Vec<Range<usize>>,
) -> GrepMatch {
    const MAX_BEFORE_TRUNC: u64 = 400;
    let fmatch = matches[0].start as u64;
    let len = match text {
        Either::Left(bytes) => bytes.len() as u64,
        Either::Right(slice) => slice.len(),
    };

    let mut start = 0u64;
    // If first match far into the line => move there
    if fmatch > MAX_BEFORE_TRUNC - (MAX_BEFORE_TRUNC / 4) {
        start = fmatch - MAX_BEFORE_TRUNC / 4;
    }
    offset += start;
    for mat in &mut matches {
        mat.start -= start as usize;
        mat.end -= start as usize;
    }

    let mut end = len;
    // If line long => shorten it
    if len - start > MAX_BEFORE_TRUNC {
        end = start + MAX_BEFORE_TRUNC;
    }

    let text = match text {
        Either::Left(bytes) => {
            // handle invalid utf8
            let pt = PieceTree::from(&bytes[start as usize..end as usize]);
            let slice = pt.slice(..);
            let slice = EndOfLine::strip_eol(&slice);
            String::from(&slice)
        }
        Either::Right(slice) => {
            let slice = slice.slice(start..end);
            let slice = EndOfLine::strip_eol(&slice);
            String::from(&slice)
        }
    };

    GrepMatch {
        line,
        text,
        matches,
        absolute_offset: Some(offset),
    }
}
