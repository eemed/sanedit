use std::{any::Any, path::PathBuf};

use sanedit_lsp::{LSPClient, LSPStartParams, Operation};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    editor::{buffers::Filetype, job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

use anyhow::Result;

use super::CHANNEL_SIZE;

/// A handle to send operations to LSP instance.
///
/// LSP is running in a job slot and communicates back using messages.
///
#[derive(Debug)]
pub(crate) struct LSPSender {
    name: String,
    sender: Sender<Operation>,
}

impl LSPSender {
    pub fn send(&mut self, op: Operation) -> Result<()> {
        self.sender.blocking_send(op)?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct LSP {
    client_id: ClientId,
    filetype: Filetype,
    working_dir: PathBuf,
}

impl LSP {
    pub fn new(id: ClientId, working_dir: PathBuf, ft: Filetype) -> LSP {
        LSP {
            client_id: id,
            filetype: ft,
            working_dir,
        }
    }
}

impl Job for LSP {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        // Clones here
        let wd = self.working_dir.clone();
        let (tx, mut rx) = channel::<Operation>(CHANNEL_SIZE);
        let ft = self.filetype.clone();

        let fut = async move {
            log::info!("Run rust-analyzer");
            let command: String = "rust-analyzer".into();
            let filetype: String = "rust".into();
            let params = LSPStartParams {
                run_command: command.clone(),
                run_args: vec![],
                root: wd,
                filetype,
            };

            let mut client = LSPClient::new(params)?;
            client.start().await;

            ctx.send(Message::Started(
                ft,
                LSPSender {
                    name: command,
                    sender: tx,
                },
            ));

            // TODO spawn reading task
            // tokio::spawn(async || {
            // });

            while let Some(op) = rx.recv().await {
                client.operate(op).await;
            }

            client.log_strerr().await;

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for LSP {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        let (win, buf) = editor.win_buf_mut(self.client_id);

        if let Ok(output) = msg.downcast::<Message>() {
            match *output {
                Message::Started(ft, mut sender) => {
                    // Send all buffers of this filetype
                    for (id, buf) in editor.buffers().iter() {
                        let is_ft = buf.filetype.as_ref().map(|f| f == &ft).unwrap_or(false);
                        if !is_ft {
                            continue;
                        }

                        let Some(path) = buf.path().map(|p| p.to_path_buf()) else {
                            continue;
                        };
                        let buf = buf.read_only_copy();
                        sender.send(Operation::DidOpen {
                            path: path.clone(),
                            buf,
                        });

                        // TODO testing
                        sender.send(Operation::Hover { path, offset: 0 });
                    }

                    // Set sender
                    editor.language_servers.insert(ft, sender);
                }
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Started(Filetype, LSPSender),
}
