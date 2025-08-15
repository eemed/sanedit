use std::any::Any;
use std::cmp::min;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use sanedit_buffer::utf8::EndOfLine;
use sanedit_buffer::{PieceTree, PieceTreeSlice, PieceTreeView};
use sanedit_core::movement::{end_of_line, start_of_line};
use sanedit_core::{Group, Item, Range, SearchMatch, Searcher};
use sanedit_utils::appendlist::Appendlist;
use sanedit_utils::sorted_vec::SortedVec;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::actions::jobs::{OptionProvider, CHANNEL_SIZE};
use crate::actions::locations;
use crate::common::Choice;

use crate::editor::ignore::Ignore;
use crate::editor::Map;
use crate::editor::{job_broker::KeepInTouch, Editor};
use sanedit_server::{ClientId, Job, JobContext, JobId, JobResult, Kill};

use super::FileOptionProvider;

#[derive(Clone)]
pub(crate) struct Grep {
    client_id: ClientId,
    pattern: String,
    file_opt_provider: FileOptionProvider,
    buffers: Arc<Map<PathBuf, PieceTreeView>>,
}

impl Grep {
    pub fn new(
        pattern: &str,
        path: &Path,
        ignore: Ignore,
        buffers: Map<PathBuf, PieceTreeView>,
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
        reader: Appendlist<Arc<Choice>>,
        write_done: Arc<AtomicUsize>,
        pattern: &str,
        msend: Sender<GrepResult>,
        buffers: Arc<Map<PathBuf, PieceTreeView>>,
        kill: Kill,
    ) {
        let Ok((searcher, _)) = Searcher::new(pattern) else {
            return;
        };
        let searcher = Arc::new(searcher);

        rayon::spawn(move || {
            let mut taken = 0;
            const INITIAL_BACKOFF: u64 = 10;
            let mut backoff = INITIAL_BACKOFF;

            loop {
                if kill.should_stop() {
                    break;
                }

                let total = write_done.load(Ordering::Acquire);
                let available = reader.len();
                let fully_read = available == total;

                // If we are done reading all available options
                if fully_read && available == taken {
                    break;
                }

                if available > taken {
                    backoff = INITIAL_BACKOFF;

                    let searcher = searcher.clone();
                    let msend = msend.clone();
                    let bufs = buffers.clone();
                    let stop = kill.clone();
                    let nreader = reader.clone();
                    taken += 1;

                    rayon::spawn(move || {
                        let opt = nreader.get(taken - 1).unwrap();
                        let path = match opt.as_ref() {
                            Choice::Path { path, .. } => path,
                            _ => {
                                return;
                            }
                        };

                        match bufs.get(path) {
                            Some(view) => {
                                // Grep editor buffers
                                Self::grep_buffer(
                                    path.clone(),
                                    view,
                                    &searcher,
                                    msend,
                                    stop.clone(),
                                );
                            }
                            None => {
                                // Grep files outside editor
                                let Ok(pt) = PieceTree::from_path(&path) else {
                                    return;
                                };
                                let view = pt.view();
                                Self::grep_buffer(
                                    path.clone(),
                                    &view,
                                    &searcher,
                                    msend,
                                    stop.clone(),
                                );
                            }
                        }
                    });
                } else {
                    thread::sleep(Duration::from_micros(backoff));
                    backoff = (backoff * 2).min(200);
                }
            }
        });
    }

    async fn send_results(mut recv: Receiver<GrepResult>, mut ctx: JobContext) {
        const FPS: Duration = Duration::from_millis(1000 / 30);
        let mut last_sent = Instant::now();
        let mut results = vec![];

        ctx.send(Start(ctx.id));

        while let Some(msg) = recv.recv().await {
            results.push(msg);

            if last_sent.elapsed() > FPS {
                ctx.send(std::mem::take(&mut results));
                last_sent = Instant::now();
            }
        }

        if !results.is_empty() {
            ctx.send(std::mem::take(&mut results));
        }
    }

    fn grep_buffer(
        path: PathBuf,
        view: &PieceTreeView,
        searcher: &Searcher,
        result_sender: Sender<GrepResult>,
        kill: Kill,
    ) {
        if !Self::should_search(view) {
            return;
        }

        let slice = view.slice(..);
        let mut matches = SortedVec::new();

        for mat in searcher.find_iter_stoppable(&slice, kill.into()) {
            let gmat = Self::prepare_match(&slice, mat);
            matches.push(gmat);
        }

        if !matches.is_empty() {
            let result = GrepResult { path, matches };
            let _ = result_sender.blocking_send(result);
        }
    }

    fn should_search(view: &PieceTreeView) -> bool {
        // Try to filter out atleast large binary files
        // Atleast 512kb to even bother with detection
        const MIN_SIZE: u64 = 1024 * 512;
        const BINARY_DETECT_WINDOW: u64 = 1024 * 8;

        if view.len() <= MIN_SIZE {
            return true;
        }

        let cap = min(BINARY_DETECT_WINDOW as u64, view.len());
        let slice = view.slice(..cap);
        let mut bytes = slice.bytes();

        while let Some(byte) = bytes.next() {
            if byte == '\0' as u8 {
                return false;
            }
        }

        true
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
            let limit = min(slice.len(), start.saturating_add(MAX_BYTES_AFTER_MATCH));
            let slice = slice.slice(..limit);
            end_of_line(&slice, start)
        };
        let line = slice.slice(sol..eol);
        let line_mat = {
            let mut range = mat.range();
            range.start -= sol;
            range.end -= sol;
            Range::from(range.start as usize..range.end as usize)
        };

        let text = {
            let line = EndOfLine::strip_eol(&line);
            let line = String::from(&line);
            // Keep byteoffset the same
            line.replace("\t", " ").replace("\n", " ")
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
            // Results channel
            let (msend, mrecv) = channel::<GrepResult>(CHANNEL_SIZE);
            let list = Appendlist::<Arc<Choice>>::new();
            let write_done = Arc::new(AtomicUsize::new(usize::MAX));

            tokio::join!(
                fopts.provide(list.clone(), ctx.kill.clone(), write_done.clone()),
                Self::grep(list, write_done, &pattern, msend, bufs, ctx.kill.clone()),
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
        if let Some(Start(id)) = msg.downcast_mut::<Start>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            win.locations.extra.is_loading = true;
            win.locations.extra.job = Some(*id);
            locations::clear_locations.execute(editor, self.client_id);
            locations::show_locations.execute(editor, self.client_id);
            return;
        }

        if let Ok(results) = msg.downcast::<Vec<GrepResult>>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            for res in results.into_iter() {
                let items: Vec<Item> = res.matches.into_iter().map(Item::from).collect();
                let mut group = Group::new(&res.path);
                items.into_iter().for_each(|i| group.push(i));
                win.locations.push(group);
            }
        }
    }

    fn on_success(&self, editor: &mut Editor) {
        let (win, _buf) = editor.win_buf_mut(self.client_id);
        win.locations.extra.is_loading = false;
        win.locations.extra.job = None;
    }

    fn on_stop(&self, editor: &mut Editor) {
        let (win, _buf) = editor.win_buf_mut(self.client_id);
        win.locations.extra.is_loading = false;
        win.locations.extra.job = None;
    }

    fn on_failure(&self, editor: &mut Editor, reason: &str) {
        log::error!("Grep error: {reason}");
        let (win, _buf) = editor.win_buf_mut(self.client_id);
        win.locations.clear();
        win.locations.extra.is_loading = false;
        win.locations.extra.job = None;
    }
}

struct Start(JobId);

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
