use std::process::Stdio;

use tokio::process::Command;

use crate::{
    editor::job_broker::KeepInTouch,
    job_runner::{BoxedJob, Job, JobContext, JobResult},
    server::ClientId,
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
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let command = self.command.clone();

        // TODO on unix create pty to run the command on
        let fut = async move {
            if let Ok(mut child) = Command::new("/bin/bash")
                .args(&["-c", &command])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
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

    fn box_clone(&self) -> BoxedJob {
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
