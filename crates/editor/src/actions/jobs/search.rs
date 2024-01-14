use std::{any::Any, cmp::min, ops::Range};

use sanedit_buffer::{ReadOnlyPieceTree, Searcher, SearcherRev};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    editor::{job_broker::KeepInTouch, windows::SearchDirection, Editor},
    server::{ClientId, Job, JobContext},
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
}

impl Search {
    pub fn forward(
        id: ClientId,
        term: &str,
        ropt: ReadOnlyPieceTree,
        range: Range<usize>,
    ) -> Search {
        Search {
            client_id: id,
            term: term.into(),
            ropt,
            range,
            dir: SearchDirection::Forward,
        }
    }

    pub fn backward(
        id: ClientId,
        term: &str,
        ropt: ReadOnlyPieceTree,
        range: Range<usize>,
    ) -> Search {
        Search {
            client_id: id,
            term: term.into(),
            ropt,
            range,
            dir: SearchDirection::Backward,
        }
    }

    async fn search_impl(
        msend: Sender<Vec<Range<usize>>>,
        dir: SearchDirection,
        term: String,
        pt: ReadOnlyPieceTree,
        range: Range<usize>,
    ) {
        let term = term.as_bytes();
        let start = range.start.saturating_sub(term.len());
        let end = min(pt.len(), range.end + term.len());
        let slice = pt.slice(start..end);

        match dir {
            SearchDirection::Forward => {
                let searcher = Searcher::new(term);
                let iter = searcher.find_iter(&slice);
                let matches: Vec<Range<usize>> = iter
                    .map(|mut range| {
                        range.start += start;
                        range.end += start;
                        range
                    })
                    .collect();
                let _ = msend.send(matches).await;
            }
            SearchDirection::Backward => {
                let searcher = SearcherRev::new(term);
                let iter = searcher.find_iter(&slice);
                let matches: Vec<Range<usize>> = iter
                    .map(|mut range| {
                        range.start += start;
                        range.end += start;
                        range
                    })
                    .collect();
                let _ = msend.send(matches).await;
            }
        };
    }

    async fn send_matches(mut ctx: JobContext, mut mrecv: Receiver<Vec<Range<usize>>>) {
        while let Some(opts) = mrecv.recv().await {
            ctx.send(SearchMessage::Matches(opts)).await;
        }
    }
}

impl Job for Search {
    fn run(&self, ctx: &crate::server::JobContext) -> crate::server::JobResult {
        let mut ctx = ctx.clone();
        let term = self.term.clone();
        let pt = self.ropt.clone();
        let range = self.range.clone();
        let dir = self.dir.clone();

        let fut = async move {
            let (msend, mrecv) = channel::<Vec<Range<usize>>>(CHANNEL_SIZE);
            tokio::join!(
                Self::search_impl(msend, dir, term, pt, range),
                Self::send_matches(ctx, mrecv),
            );
            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> crate::server::BoxedJob {
        Box::new((*self).clone())
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
