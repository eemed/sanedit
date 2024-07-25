use std::{any::Any, ops::Range};

use sanedit_buffer::ReadOnlyPieceTree;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    common::search::PTSearcher,
    editor::{
        job_broker::KeepInTouch,
        windows::{SearchDirection, SearchKind},
        Editor,
    },
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

use super::CHANNEL_SIZE;

enum SearchMessage {
    Matches(Vec<Range<usize>>),
}

#[derive(Clone)]
pub(crate) struct Search {
    client_id: ClientId,
    term: String,
    ropt: ReadOnlyPieceTree,
    range: Range<usize>,
    dir: SearchDirection,
    kind: SearchKind,
}

impl Search {
    pub fn new(
        id: ClientId,
        term: &str,
        ropt: ReadOnlyPieceTree,
        range: Range<usize>,
        dir: SearchDirection,
        kind: SearchKind,
    ) -> Search {
        Search {
            client_id: id,
            term: term.into(),
            ropt,
            range,
            dir,
            kind,
        }
    }

    // async fn search_regex_impl(
    //     msend: Sender<Vec<Range<usize>>>,
    //     dir: SearchDirection,
    //     term: String,
    //     pt: ReadOnlyPieceTree,
    //     range: Range<usize>,
    // ) {
    //     if term.is_empty() {
    //         return;
    //     }

    //     let start = 0;
    //     let len = pt.len();
    //     let chunks = pt.chunks();
    //     let chunk = chunks.get();
    //     let cursor = PTRegexCursor { len, chunks, chunk };
    //     let input = Input::new(cursor);

    //     match Regex::new(&term) {
    //         Ok(regex) => {
    //             let iter = regex.find_iter(input);
    //             let matches: Vec<Range<usize>> = iter
    //                 .map(|mat| {
    //                     let mut range = mat.range();
    //                     range.start += start;
    //                     range.end += start;
    //                     range
    //                 })
    //                 .collect();
    //             let _ = msend.send(matches).await;
    //         }
    //         Err(e) => {
    //             log::error!("Invalid regex: {e}");
    //             return;
    //         }
    //     }
    // }

    // async fn search_impl(
    //     msend: Sender<Vec<Range<usize>>>,
    //     dir: SearchDirection,
    //     term: String,
    //     pt: ReadOnlyPieceTree,
    //     range: Range<usize>,
    // ) {
    //     let term = term.as_bytes();
    //     let start = range.start.saturating_sub(term.len());
    //     let end = min(pt.len(), range.end + term.len());
    //     let slice = pt.slice(start..end);

    //     match dir {
    //         SearchDirection::Forward => {
    //             let searcher = Searcher::new(term);
    //             let iter = searcher.find_iter(&slice);
    //             let matches: Vec<Range<usize>> = iter
    //                 .map(|mut range| {
    //                     range.start += start;
    //                     range.end += start;
    //                     range
    //                 })
    //                 .collect();
    //             let _ = msend.send(matches).await;
    //         }
    //         SearchDirection::Backward => {
    //             let searcher = SearcherRev::new(term);
    //             let iter = searcher.find_iter(&slice);
    //             let matches: Vec<Range<usize>> = iter
    //                 .map(|mut range| {
    //                     range.start += start;
    //                     range.end += start;
    //                     range
    //                 })
    //                 .collect();
    //             let _ = msend.send(matches).await;
    //         }
    //     };
    // }

    async fn search(
        msend: Sender<Vec<Range<usize>>>,
        searcher: PTSearcher,
        ropt: ReadOnlyPieceTree,
        view: Range<usize>,
    ) {
        let slice = ropt.slice(view);
        let start = slice.start();

        let matches = searcher
            .find_iter(&slice)
            .map(|mat| {
                let mut range = mat.range();
                range.start += start;
                range.end += start;
                range
            })
            .collect();
        let _ = msend.send(matches).await;
    }

    async fn send_matches(mut ctx: JobContext, mut mrecv: Receiver<Vec<Range<usize>>>) {
        while let Some(opts) = mrecv.recv().await {
            ctx.send(SearchMessage::Matches(opts));
        }
    }
}

impl Job for Search {
    fn run(&self, ctx: JobContext) -> JobResult {
        let term = self.term.clone();
        let pt = self.ropt.clone();
        let range = self.range.clone();
        let dir = self.dir.clone();
        let kind = self.kind.clone();

        let fut = async move {
            let (msend, mrecv) = channel::<Vec<Range<usize>>>(CHANNEL_SIZE);
            let searcher = PTSearcher::new(&term, dir, kind)?;
            tokio::join!(
                Self::search(msend, searcher, pt, range),
                Self::send_matches(ctx, mrecv),
            );
            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for Search {
    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(output) = msg.downcast::<SearchMessage>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            match *output {
                SearchMessage::Matches(matches) => win.search.hl_matches = matches,
            }
        }
    }

    fn client_id(&self) -> ClientId {
        self.client_id
    }
}
