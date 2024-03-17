use std::{any::Any, ops::Range};

use sanedit_buffer::ReadOnlyPieceTree;

use crate::{
    editor::{buffers::BufferId, job_broker::KeepInTouch, Editor},
    server::{ClientId, Job, JobContext, JobResult},
    syntax::{Syntax, SyntaxParseResult},
};

#[derive(Clone)]
pub(crate) struct SyntaxParser {
    client_id: ClientId,
    syntax: Syntax,
    bid: BufferId,
    ropt: ReadOnlyPieceTree,
    range: Range<usize>,
}

impl SyntaxParser {
    pub fn new(
        id: ClientId,
        bid: BufferId,
        syntax: Syntax,
        ropt: ReadOnlyPieceTree,
        range: Range<usize>,
    ) -> Self {
        SyntaxParser {
            client_id: id,
            bid,
            syntax,
            ropt,
            range,
        }
    }
}

impl Job for SyntaxParser {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let bid = self.bid.clone();
        let pt = self.ropt.clone();
        let range = self.range.clone();
        let syntax = self.syntax.clone();

        let fut = async move {
            let ast = syntax.parse(bid, &pt, range);
            ctx.send(ast).await;
            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> crate::server::BoxedJob {
        Box::new((*self).clone())
    }
}

impl KeepInTouch for SyntaxParser {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(output) = msg.downcast::<SyntaxParseResult>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            win.on_syntax_parsed(*output);
        }
    }

    fn on_success(&self, editor: &mut Editor) {}

    fn on_failure(&self, editor: &mut Editor, reason: &str) {}
}
