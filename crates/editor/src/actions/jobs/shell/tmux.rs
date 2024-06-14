use std::process::Command;

use sanedit_buffer::ReadOnlyPieceTree;

use crate::{editor::job_broker::KeepInTouch, job_runner::Job, server::ClientId};

#[derive(Clone)]
pub(crate) struct TmuxShellCommand {
    client_id: ClientId,
    command: String,
    pipe_input: Option<ReadOnlyPieceTree>,
    shell: String,
    pane_id: Option<usize>,
}

impl TmuxShellCommand {
    pub fn new(id: ClientId, command: &str) -> TmuxShellCommand {
        TmuxShellCommand {
            client_id: id,
            command: command.into(),
            pipe_input: None,
            shell: "/bin/bash".into(),
            pane_id: None,
        }
    }
}

impl Job for TmuxShellCommand {
    fn run(&self, ctx: crate::job_runner::JobContext) -> crate::job_runner::JobResult {
        let command = self.command.clone();
        let ropt = self.pipe_input.clone();
        let shell = self.shell.clone();
        let pane_id = self.pane_id.clone();

        let fut = async move {
            let output = Command::new(&shell)
                .args(&[
                    "-c",
                    &format!("tmux split-window -l 15% -d -P -F \"#{{pane_id}}\" \" {command} ; {shell}\""),
                ])
                .output()?;

            let pane: usize = {
                // Returns "%dd\n" where dd are the pane number
                let pane = std::str::from_utf8(&output.stdout)?.trim();
                pane[1..].parse()?
            };

            log::info!("pane: {pane}");

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for TmuxShellCommand {
    fn client_id(&self) -> ClientId {
        self.client_id
    }
}
