use std::{any::Any, path::PathBuf};

use sanedit_lsp::{LSPClient, LSPStartParams, Operation};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    editor::{buffers::Filetype, job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

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
        let (tx, rx) = channel::<Operation>(CHANNEL_SIZE);
        let ft = self.filetype.clone();

        let fut = async move {
            log::info!("Run rust-analyzer");
            let command: String = "rust-analyzer".into();
            let params = LSPStartParams {
                run_command: command.clone(),
                run_args: vec![],
                root: wd,
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
                Message::Started(ft, send) => {
                    editor.language_servers.insert(ft, send);
                }
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Started(Filetype, LSPSender),
}
