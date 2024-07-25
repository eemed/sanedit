mod tmux;

use std::process::Stdio;

use sanedit_buffer::ReadOnlyPieceTree;
use tokio::process::Command;

use crate::{
    editor::job_broker::KeepInTouch,
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

pub(crate) use tmux::*;

#[derive(Clone)]
pub(crate) struct ShellCommand {
    client_id: ClientId,
    command: String,
    pipe_input: Option<ReadOnlyPieceTree>,
}

impl ShellCommand {
    pub fn new(client_id: ClientId, command: &str) -> ShellCommand {
        ShellCommand {
            client_id,
            command: command.into(),
            pipe_input: None,
        }
    }

    pub fn pipe(mut self, ropt: ReadOnlyPieceTree) -> Self {
        self.pipe_input = Some(ropt);
        self
    }
}

impl Job for ShellCommand {
    fn run(&self, ctx: JobContext) -> JobResult {
        let command = self.command.clone();
        let ropt = self.pipe_input.clone();

        let fut = async move {
            let mut cmd = Command::new("/bin/sh");

            cmd.args(&["-c", &format!("setsid {}", command)])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if ropt.is_some() {
                cmd.stdin(Stdio::piped());
            } else {
                cmd.stdin(Stdio::null());
            }

            if let Ok(child) = cmd.spawn() {
                if let Ok(output) = child.wait_with_output().await {
                    log::info!(
                        "Ran '{}', stdout: {}, stderr: {}",
                        command,
                        std::str::from_utf8(&output.stdout).unwrap(),
                        std::str::from_utf8(&output.stderr).unwrap(),
                    )
                }
            }
            Ok(())
        };

        Box::pin(fut)
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
