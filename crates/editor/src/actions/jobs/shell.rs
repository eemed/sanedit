use tokio::process::Command;

use crate::{
    editor::job_broker::KeepInTouch,
    server::{ClientId, Job},
};

#[derive(Clone)]
pub(crate) struct ShellCommand {
    client_id: ClientId,
    command: String,
}

impl Job for ShellCommand {
    fn run(&self, ctx: &crate::server::JobContext) -> crate::server::JobResult {
        let mut ctx = ctx.clone();
        let cmd = self.command.clone();

        let fut = async move {
            if let Ok(splits) = shellwords::split(&cmd) {
                todo!()
            }
            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> crate::server::BoxedJob {
        Box::new((*self).clone())
    }
}

impl KeepInTouch for ShellCommand {
    fn on_message(&self, editor: &mut crate::editor::Editor, msg: Box<dyn std::any::Any>) {
        todo!()
    }

    fn client_id(&self) -> ClientId {
        self.client_id
    }
}
