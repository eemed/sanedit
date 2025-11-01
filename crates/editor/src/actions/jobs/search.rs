use std::{
    any::Any,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
};

use sanedit_buffer::PieceTreeSlice;
use sanedit_core::BufferRange;

use sanedit_syntax::PieceTreeSliceSource;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::editor::{
    buffers::BufferId, job_broker::KeepInTouch, windows::SearchHighlights, Editor,
};
use sanedit_server::{ClientId, Job, JobContext, JobResult};

use super::CHANNEL_SIZE;
use sanedit_core::Searcher;

enum SearchMessage {
    Matches(Vec<BufferRange>),
}

#[derive(Clone)]
pub(crate) struct Search {
    client_id: ClientId,
    searcher: Arc<Searcher>,
    slice: PieceTreeSlice,
    bid: BufferId,
    changes_made: u32,
}

impl Search {
    pub fn new(
        id: ClientId,
        searcher: Searcher,
        bid: BufferId,
        slice: PieceTreeSlice,
        changes_made: u32,
    ) -> Search {
        Search {
            client_id: id,
            searcher: Arc::new(searcher),
            bid,
            slice,
            changes_made,
        }
    }

    async fn search(
        msend: Sender<Vec<BufferRange>>,
        searcher: Arc<Searcher>,
        slice: PieceTreeSlice,
        _stop: Arc<AtomicBool>,
    ) {
        let start = slice.start();
        let Ok(mut source) = PieceTreeSliceSource::new(&slice) else {
            return;
        };

        let matches = searcher
            .find_iter(&mut source)
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
        let pt = self.slice.clone();
        let searcher = self.searcher.clone();

        let fut = async move {
            let (msend, mrecv) = channel::<Vec<BufferRange>>(CHANNEL_SIZE);
            tokio::join!(
                Self::search(msend, searcher, pt, ctx.kill.clone().into()),
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
            let (win, buf) = editor.win_buf_mut(self.client_id);
            if buf.id != self.bid || buf.total_changes_made() != self.changes_made {
                return;
            }

            match *output {
                SearchMessage::Matches(matches) => {
                    win.search.set_highlights(SearchHighlights {
                        highlights: matches.into(),
                        changes_made: self.changes_made,
                        buffer_range: BufferRange::from_bounds(
                            self.slice.start()..self.slice.end(),
                        ),
                    });
                }
            }
        }
    }

    fn client_id(&self) -> ClientId {
        self.client_id
    }
}
