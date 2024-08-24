use std::{any::Any, path::PathBuf};

use crate::{
    editor::{buffers::Filetype, job_broker::KeepInTouch, options::LSPOptions, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};
use sanedit_lsp::{LSPClientParams, LSPClientSender, Request, Response};

use anyhow::Result;
use sanedit_messages::redraw::{Severity, StatusMessage};

/// A handle to send operations to LSP instance.
///
/// LSP is running in a job slot and communicates back using messages.
///
#[derive(Debug)]
pub(crate) struct LSPSender {
    /// Name of the LSP server
    name: String,

    /// Root where LSP is started
    root: PathBuf,

    /// Client to send messages to LSP server
    sender: LSPClientSender,
}

impl LSPSender {
    pub fn send(&mut self, op: Request) -> Result<()> {
        self.sender.send(op);
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct LSP {
    client_id: ClientId,
    filetype: Filetype,
    working_dir: PathBuf,
    opts: LSPOptions,
}

impl LSP {
    pub fn new(id: ClientId, working_dir: PathBuf, ft: Filetype, opts: &LSPOptions) -> LSP {
        LSP {
            client_id: id,
            filetype: ft,
            working_dir,
            opts: opts.clone(),
        }
    }
}

impl Job for LSP {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        // Clones here
        let wd = self.working_dir.clone();
        let ft = self.filetype.clone();
        let opts = self.opts.clone();

        let fut = async move {
            log::info!("Run rust-analyzer");
            let LSPOptions { command, args } = opts;
            let filetype: String = ft.as_str().into();
            let params = LSPClientParams {
                run_command: command.clone(),
                run_args: args,
                root: wd.clone(),
                filetype,
            };

            let (sender, mut reader) = params.spawn().await?;
            log::info!("Client started");

            let sender = LSPSender {
                name: command,
                sender,
                root: wd,
            };
            ctx.send(Message::Started(ft, sender));

            while let Some(response) = reader.recv().await {
                ctx.send(Message::Response(response));
            }

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
                        let ro = buf.read_only_copy();
                        sender.send(Request::DidOpen {
                            path: path.clone(),
                            buf: ro,
                        });
                    }

                    // Set sender
                    editor.language_servers.insert(ft, sender);
                }
                Message::Response(response) => match response {
                    Response::Request(request) => match request {
                        sanedit_lsp::RequestResult::Hover { text, position } => {
                            win.popup = Some(StatusMessage {
                                severity: Severity::Info,
                                message: text,
                            });
                        }
                        sanedit_lsp::RequestResult::GotoDefinition { path, position } => {
                            if editor.open_file(self.client_id, path).is_ok() {
                                let (win, buf) = editor.win_buf_mut(self.client_id);
                                let offset = position.to_offset(&buf.read_only_copy());
                                win.goto_offset(offset, buf);
                            }
                        }
                        sanedit_lsp::RequestResult::Complete {
                            path,
                            position,
                            results,
                        } => todo!(),
                    },
                },
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Started(Filetype, LSPSender),
    Response(Response),
}
