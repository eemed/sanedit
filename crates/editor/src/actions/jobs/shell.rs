use std::process::Stdio;

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

impl ShellCommand {
    pub fn new(client_id: ClientId, command: &str) -> ShellCommand {
        ShellCommand {
            client_id,
            command: command.into(),
        }
    }
}

impl Job for ShellCommand {
    fn run(&self, ctx: &crate::server::JobContext) -> crate::server::JobResult {
        let mut ctx = ctx.clone();
        let command = self.command.clone();

        let fut = async move {
            if let Ok(cmd) = shellwords::split(&command) {
                if let Some((cmd, opts)) = cmd.split_first() {
                    if let Ok(mut child) =
                        Command::new(cmd).args(opts).stdout(Stdio::null()).spawn()
                    {
                        if let Ok(status) = child.wait().await {
                            log::info!(
                                "Ran '{}', Output: {}",
                                cmd,
                                status.success(),
                                // std::str::from_utf8(&output.stdout).unwrap()
                            )
                        }
                    }
                }
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
