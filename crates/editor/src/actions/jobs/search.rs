use std::any::Any;

use sanedit_buffer::PieceTreeView;
use sanedit_core::{BufferRange, SearchDirection, SearchKind};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::editor::{job_broker::KeepInTouch, Editor};
use sanedit_server::{ClientId, Job, JobContext, JobResult};

use super::CHANNEL_SIZE;
use sanedit_core::PTSearcher;

enum SearchMessage {
    Matches(Vec<BufferRange>),
}

#[derive(Clone)]
pub(crate) struct Search {
    client_id: ClientId,
    term: String,
    ropt: PieceTreeView,
    range: BufferRange,
    dir: SearchDirection,
    kind: SearchKind,
}

impl Search {
    pub fn new(
        id: ClientId,
        term: &str,
        ropt: PieceTreeView,
        range: BufferRange,
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

    async fn search(
        msend: Sender<Vec<BufferRange>>,
        searcher: PTSearcher,
        ropt: PieceTreeView,
        view: BufferRange,
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

    async fn send_matches(mut ctx: JobContext, mut mrecv: Receiver<Vec<BufferRange>>) {
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
            let (msend, mrecv) = channel::<Vec<BufferRange>>(CHANNEL_SIZE);
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
