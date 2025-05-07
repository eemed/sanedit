use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustc_hash::FxHashMap;
use sanedit_buffer::utf8::EndOfLine;
use sanedit_buffer::{PieceTree, PieceTreeSlice, PieceTreeView};
use sanedit_core::movement::{end_of_line, start_of_line};
use sanedit_core::{Group, Item, Range, SearchKind, SearchMatch, Searcher};
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
        let searcher = Arc::new(Searcher::new(pattern).expect("Cannot build Searcher"));

        while let Some(opt) = orecv.recv().await {
            let searcher = searcher.clone();
            let msend = msend.clone();
            let bufs = buffers.clone();

            rayon::spawn(move || {
                let path = match opt.as_ref() {
                    Choice::Path { path, .. } => path,
                    _ => return,
                };

                match bufs.get(path) {
                    Some(view) => {
                        // Grep editor buffers
                        Self::grep_buffer(path.clone(), view, &searcher, msend);
                    }
                    None => {
                        // Grep files outside editor
                        let Ok(pt) = PieceTree::from_path(&path) else {
                            return;
                        };
                        let view = pt.view();
                        Self::grep_buffer(path.clone(), &view, &searcher, msend);
                    }
                }
            });
        }
    }

    async fn send_results(mut recv: Receiver<GrepResult>, mut ctx: JobContext) {
        ctx.send(Start);

        while let Some(msg) = recv.recv().await {
            ctx.send(msg);
        }
    }

    fn grep_buffer(
        path: PathBuf,
        view: &PieceTreeView,
        searcher: &Searcher,
        result_sender: Sender<GrepResult>,
    ) {
        let slice = view.slice(..);
        let mut matches = SortedVec::new();

        for mat in searcher.find_iter(&slice) {
            let gmat = Self::prepare_match(&slice, mat);
            matches.push(gmat);
        }

        if !matches.is_empty() {
            let result = GrepResult { path, matches };
            let _ = result_sender.blocking_send(result);
        }
    }

    fn prepare_match(slice: &PieceTreeSlice, mat: SearchMatch) -> GrepMatch {
        const MAX_BYTES_BEFORE_MATCH: u64 = 128;
        const MAX_BYTES_AFTER_MATCH: u64 = 256;

        let start = mat.range().start;

        let sol = {
            let limit = start.saturating_sub(MAX_BYTES_BEFORE_MATCH);
            let mat_start = start - limit;
            let slice = slice.slice(limit..);
            start_of_line(&slice, mat_start) + limit
        };

        let eol = {
            let limit = start.saturating_add(MAX_BYTES_AFTER_MATCH);
            let slice = slice.slice(..limit);
            end_of_line(&slice, start)
        };
        let line = slice.slice(sol..eol);
        let line_mat = {
            let mut range = mat.range();
            range.start -= sol;
            range.end -= sol;
            Range::new(range.start as usize, range.end as usize)
        };

        let text = {
            let line = EndOfLine::strip_eol(&line);
            String::from(&line)
        };

        GrepMatch {
            line: None,
            text,
            matches: vec![line_mat],
            absolute_offset: Some(line.start()),
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
