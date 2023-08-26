use std::{any::Any, cmp::min, ops::Range, sync::Arc};

use sanedit_buffer::{ReadOnlyPieceTree, Searcher, SearcherRev};
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use crate::{
    editor::{jobs::Job, windows::SearchDirection, Editor},
    server::{ClientId, JobFutureFn, JobProgress, JobProgressSender},
};

enum SearchResult {
    Matches(Vec<Range<usize>>),
    Reset,
}

pub(crate) fn search(editor: &mut Editor, id: ClientId, term_in: Receiver<String>) -> Job {
    let (win, buf) = editor.win_buf_mut(id);
    let dir = win.search.direction;
    let ropt = buf.read_only_copy();
    let view = win.view().range();

    let fun: JobFutureFn =
        { Box::new(move |send| Box::pin(search_impl(dir, ropt, view, send, term_in))) };
    let on_output = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {
        if let Ok(output) = out.downcast::<SearchResult>() {
            let (win, _buf) = editor.win_buf_mut(id);
            match *output {
                SearchResult::Matches(matches) => win.search.hl_matches = matches,
                SearchResult::Reset => win.search.hl_matches.clear(),
            }
        }
    });
    let on_error = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {});
    Job::new(id, fun).on_output(on_output).on_error(on_error)
}

async fn search_impl(
    dir: SearchDirection,
    ropt: ReadOnlyPieceTree,
    view: Range<usize>,
    out: JobProgressSender,
    mut term_in: Receiver<String>,
) -> bool {
    let mut handle: Option<JoinHandle<()>> = None;

    while let Some(term) = term_in.recv().await {
        if term.is_empty() {
            continue;
        }

        if let Some(h) = handle.take() {
            h.abort();
            h.await;
        }

        let pt = ropt.clone();
        let mut out = out.clone();

        let join = tokio::spawn(async move {
            let term = term.as_bytes();
            let start = view.start.saturating_sub(term.len());
            let end = min(pt.len(), view.end + term.len());
            let slice = pt.slice(start..end);

            match dir {
                SearchDirection::Forward => {
                    let searcher = Searcher::new(term);
                    let mut iter = searcher.find_iter(&slice);
                    let matches: Vec<Range<usize>> = iter.collect();
                    out.send(JobProgress::Output(Box::new(SearchResult::Matches(
                        matches,
                    ))))
                    .await;
                }
                SearchDirection::Backward => {
                    let searcher = SearcherRev::new(term);
                    let mut iter = searcher.find_iter(&slice);
                    let matches: Vec<Range<usize>> = iter.collect();
                    out.send(JobProgress::Output(Box::new(SearchResult::Matches(
                        matches,
                    ))))
                    .await;
                }
            };
        });

        handle = Some(join);
    }

    true
}
