use std::any::Any;

use sanedit_buffer::PieceTreeView;
use sanedit_core::BufferRange;

use crate::editor::{
    buffers::BufferId,
    job_broker::KeepInTouch,
    syntax::{Syntax, SyntaxResult},
    windows::ViewSyntax,
    Editor,
};
use sanedit_server::{CPUJob, ClientId, JobContext};

#[derive(Clone)]
pub(crate) struct SyntaxParser {
    client_id: ClientId,
    syntax: Syntax,
    bid: BufferId,
    total_changes_made: u32,
    pt: PieceTreeView,
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
            pt: ropt,
            range,
        }
    }
}

impl CPUJob for SyntaxParser {
    fn run(&self, mut ctx: JobContext) -> anyhow::Result<()> {
        let ast = self
            .syntax
            .parse(&self.pt, self.range.clone(), ctx.kill.clone())?;
        ctx.send(ast);
        Ok(())
    }
}

impl KeepInTouch for SyntaxParser {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(output) = msg.downcast::<SyntaxResult>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            *win.view_syntax() = ViewSyntax::new(self.bid, *output, self.total_changes_made);
        }
    }
}
