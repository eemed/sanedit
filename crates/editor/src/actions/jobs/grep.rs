use std::any::Any;
use std::cmp::min;

use std::fs::File;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rayon::iter::{ParallelBridge as _, ParallelIterator as _};
use sanedit_buffer::{PieceTree, PieceTreeSlice};
use sanedit_core::movement::{end_of_line, start_of_line};
use sanedit_core::{Group, Item, Range, SearchMatch, Searcher};

use sanedit_syntax::{BufferedSource, PieceTreeSliceSource, Source};
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
    buffers: Arc<Map<PathBuf, PieceTreeSlice>>,
}

impl Grep {
    pub fn new(
        pattern: &str,
        path: &Path,
        ignore: Ignore,
        buffers: Map<PathBuf, PieceTreeSlice>,
        id: ClientId,
        git_ignore: bool,
    ) -> Grep {
        let fprovider = FileOptionProvider::new(path, ignore, git_ignore);

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
        modified_buffers: Arc<Map<PathBuf, PieceTreeSlice>>,
        kill: Kill,
    ) {
        let Ok((searcher, _)) = Searcher::new(pattern) else {
            return;
        };
        let searcher = Arc::new(searcher);

        let (tx, rx) = crossbeam::channel::unbounded::<usize>();
        let kill_p = kill.clone();
        let reader_p = reader.clone();

        let producer = tokio::task::spawn(async move {
            let mut next = 0;

            loop {
                if kill_p.should_stop() {
                    break;
                }

                let available = reader_p.len();
                let total = write_done.load(Ordering::Acquire);

                // push new work indices
                while next < available {
                    if tx.send(next).is_err() {
                        return;
                    }
                    next += 1;
                }

                if next == total && available == total {
                    break;
                }

                tokio::task::yield_now().await;
            }
        });

        let searcher2 = searcher.clone();
        let msend2 = msend.clone();
        let modified2 = modified_buffers.clone();
        let kill2 = kill.clone();

        let worker = tokio::task::spawn_blocking(move || {
            rx.into_iter().par_bridge().for_each(|idx| {
                if kill2.should_stop() {
                    return;
                }

                let choice = reader.get(idx).unwrap();

                let path = match choice.as_ref() {
                    Choice::Path { path, .. } => path,
                    _ => return,
                };

                if let Some(slice) = modified2.get(path) {
                    Self::grep_buffer(
                        path.clone(),
                        slice,
                        &searcher2,
                        msend2.clone(),
                        kill2.clone(),
                    );
                } else {
                    Self::grep_file(path.clone(), &searcher2, msend2.clone(), kill2.clone());
                }
            });
        });

        let _ = producer.await;
        let _ = worker.await;
    }

    async fn send_results(mut recv: Receiver<GrepResult>, ctx: JobContext) {
        const FPS: Duration = Duration::from_millis(1000 / 5);
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

    fn grep_file(
        path: PathBuf,
        searcher: &Searcher,
        result_sender: Sender<GrepResult>,
        kill: Kill,
    ) {
        if !Self::should_search_file(path.as_path()) {
            return;
        }

        let Ok(file) = File::open(&path) else {
            return;
        };
        let Ok(mut source) = BufferedSource::with_stop(file, kill.into()) else {
            return;
        };

        let results = {
            let mut results: Vec<SearchMatch> = vec![];
            for mat in searcher.find_iter(&mut source) {
                results.push(mat);
            }
            results
        };

        let mut matches: SortedVec<GrepMatch> = SortedVec::new();
        for mat in results {
            if let Some(gmat) = Self::prepare_match(&mut source, mat) {
                // If already got this offset, combine them
                if let Some(mut prev) = matches.pop() {
                    if prev.absolute_offset == gmat.absolute_offset {
                        prev.matches.extend(gmat.matches);
                        matches.push(prev);
                        continue;
                    } else {
                        matches.push(prev);
                    }
                }

                matches.push(gmat);
            }
        }

        if !matches.is_empty() {
            let result = GrepResult { path, matches };
            let _ = result_sender.blocking_send(result);
        }
    }

    fn grep_buffer(
        path: PathBuf,
        slice: &PieceTreeSlice,
        searcher: &Searcher,
        result_sender: Sender<GrepResult>,
        kill: Kill,
    ) {
        if !Self::should_search(slice) {
            return;
        }

        let Ok(mut source) = PieceTreeSliceSource::with_stop(slice, kill.into()) else {
            return;
        };

        let results = {
            let mut results = vec![];
            for mat in searcher.find_iter(&mut source) {
                results.push(mat);
            }
            results
        };

        let mut matches = SortedVec::new();
        for mat in results {
            if let Some(gmat) = Self::prepare_match(&mut source, mat) {
                matches.push(gmat);
            }
        }

        if !matches.is_empty() {
            let result = GrepResult { path, matches };
            let _ = result_sender.blocking_send(result);
        }
    }

    fn should_search_file(path: &Path) -> bool {
        // Try to filter out atleast large binary files
        // Atleast 512kb to even bother with detection
        const MIN_SIZE: u64 = 1024 * 512;
        const BINARY_DETECT_WINDOW: u64 = 1024 * 8;

        let len = path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        if len <= MIN_SIZE {
            return true;
        }

        let cap = min(BINARY_DETECT_WINDOW, len);
        let mut buf = vec![0u8; BINARY_DETECT_WINDOW as usize].into_boxed_slice();
        let Ok(mut file) = File::open(path) else {
            return false;
        };
        let mut n = 0;
        while n < cap {
            match file.read(&mut buf) {
                Ok(read) => {
                    if read == 0 {
                        break;
                    }
                    n += read as u64;
                }
                Err(_) => return false,
            }
        }

        for i in 0..(n as usize) {
            let byte = buf[i];
            if byte == b'\0' {
                return false;
            }
        }

        true
    }

    fn should_search(view: &PieceTreeSlice) -> bool {
        // Try to filter out atleast large binary files
        // Atleast 512kb to even bother with detection
        const MIN_SIZE: u64 = 1024 * 512;
        const BINARY_DETECT_WINDOW: u64 = 1024 * 8;

        if view.len() <= MIN_SIZE {
            return true;
        }

        let cap = min(BINARY_DETECT_WINDOW, view.len());
        let slice = view.slice(..cap);
        let mut bytes = slice.bytes();

        while let Some(byte) = bytes.next() {
            if byte == b'\0' {
                return false;
            }
        }

        true
    }
    const MAX_BYTES_BEFORE_MATCH: u64 = 128;
    const MAX_BYTES_AFTER_MATCH: u64 = 128;

    fn prepare_match<S: Source>(source: &mut S, mat: SearchMatch) -> Option<GrepMatch> {
        let start = mat.range().start;
        let start_limit = start.saturating_sub(Self::MAX_BYTES_BEFORE_MATCH);
        let end = mat.range().end;
        let end_limit = min(
            source.len(),
            end.saturating_add(Self::MAX_BYTES_AFTER_MATCH),
        );
        let bytes = source.slice(start_limit..end_limit)?;

        // Just create piecetree for convinience
        let pt = PieceTree::from(bytes);
        let slice = pt.slice(..);

        let relative_mat_start = start - start_limit;
        let sol = start_of_line(&slice, relative_mat_start);
        let relative_mat_end = relative_mat_start + mat.range().len();
        let eol = end_of_line(&slice, relative_mat_end);
        let line = slice.slice(sol..eol);
        let line_mat = {
            let mut range = mat.range();
            range.start -= start_limit + sol;
            range.end -= start_limit + sol;
            Range::from(range.start as usize..range.end as usize)
        };

        Some(GrepMatch {
            line: None,
            text: String::from(&line),
            matches: vec![line_mat],
            absolute_offset: Some(line.start() + start_limit),
        })
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
            win.locations.extra.title = format!("Grep {:?}", self.pattern);
            locations::clear_locations.execute(editor, self.client_id);
            locations::show_locations.execute(editor, self.client_id);
            return;
        }

        if let Ok(results) = msg.downcast::<Vec<GrepResult>>() {
            let draw = editor.draw_state(self.client_id);
            draw.no_redraw_window();

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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GrepMatch {
    absolute_offset: Option<u64>,
    line: Option<u64>,
    /// Matches found in text
    matches: Vec<Range<usize>>,
    text: String,
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
