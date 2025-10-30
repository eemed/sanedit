use std::process::Stdio;


use sanedit_buffer::PieceTreeSlice;use tokio::process::Command;

use sanedit_server::{ClientId, Job, JobContext, JobResult};

#[derive(Clone)]
pub(crate) struct ShellCommand {
    _client_id: ClientId,
    command: String,
    pipe_input: Option<PieceTreeSlice>,
}

#[allow(dead_code)]
impl ShellCommand {
    pub fn new(client_id: ClientId, command: &str) -> ShellCommand {
        ShellCommand {
            _client_id: client_id,
            command: command.into(),
            pipe_input: None,
        }
    }

    pub fn pipe(mut self, slice: PieceTreeSlice) -> Self {
        self.pipe_input = Some(slice);
        self
    }
}

impl Job for ShellCommand {
    fn run(&self, _ctx: JobContext) -> JobResult {
        let command = self.command.clone();
        let ropt = self.pipe_input.clone();

        let fut = async move {
            let mut cmd = Command::new("/bin/sh");

            cmd.args(["-c", &format!("setsid {}", command)])
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
