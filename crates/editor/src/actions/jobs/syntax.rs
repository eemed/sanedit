use std::any::Any;

use sanedit_buffer::PieceTreeView;

use crate::{
    editor::{
        buffers::{BufferId, BufferRange},
        job_broker::{CPUJob, KeepInTouch},
        syntax::{Syntax, SyntaxParseResult},
        Editor,
    },
    job_runner::JobContext,
    server::ClientId,
};

#[derive(Clone)]
pub(crate) struct SyntaxParser {
    client_id: ClientId,
    syntax: Syntax,
    bid: BufferId,
    total_changes_made: u32,
    ropt: PieceTreeView,
    range: BufferRange,
}

impl SyntaxParser {
    pub fn new(
        id: ClientId,
        bid: BufferId,
        total_changes_made: u32,
        syntax: Syntax,
        ropt: PieceTreeView,
        range: BufferRange,
    ) -> Self {
        SyntaxParser {
            client_id: id,
            bid,
            total_changes_made,
            syntax,
            ropt,
            range,
        }
    }
}

impl CPUJob for SyntaxParser {
    fn run(&self, mut ctx: JobContext) -> anyhow::Result<()> {
        let ast = self.syntax.parse(
            self.bid,
            self.total_changes_made,
            &self.ropt,
            self.range.clone(),
            ctx.kill.subscribe(),
        )?;
        ctx.send(ast);
        Ok(())
    }
}

impl KeepInTouch for SyntaxParser {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(output) = msg.downcast::<SyntaxParseResult>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            *win.syntax_result() = *output;
        }
    }
}
