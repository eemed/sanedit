use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::{path::PathBuf, process::Stdio};

use crate::jsonrpc::{JsonNotification, JsonRequest};
use crate::process::{ProcessHandler, ServerRequest};
use crate::util::path_to_uri;
use crate::{Change, Position, Request, RequestResult, Response};

use anyhow::{anyhow, Result};
use sanedit_buffer::ReadOnlyPieceTree;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::{io::BufReader, process::Command};

/// a struct to put all the parameters
#[derive(Clone)]
pub struct LSPClientParams {
    pub run_command: String,
    pub run_args: Vec<String>,
    pub root: PathBuf,
    pub filetype: String,
}

impl LSPClientParams {
    pub async fn spawn(self) -> Result<(LSPClientSender, LSPClientReader)> {
        // Spawn server
        let mut cmd = Command::new(&self.run_command)
            .args(&*self.run_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = cmd.stdin.take().ok_or(anyhow!("Failed to take stdin"))?;
        let stdout = BufReader::new(cmd.stdout.take().ok_or(anyhow!("Failed to take stdout"))?);
        let stderr = BufReader::new(cmd.stderr.take().ok_or(anyhow!("Failed to take stderr"))?);
        let (server_sender, server_recv) = channel(256);
        let (req_sender, req_receiver) = channel(256);
        let (res_sender, res_receiver) = channel(256);
        let (init_send, init_recv) = oneshot::channel();

        let params = Arc::new(self);
        let server = ProcessHandler {
            params: params.clone(),
            _process: cmd,
            stdin,
            stdout,
            stderr,
            receiver: server_recv,
            initialized: Some(init_send),
            in_flight: BTreeMap::default(),
        };

        tokio::spawn(server.run());

        // Wait for initialization
        let init_params = init_recv.await??;

        let client = LSPClient {
            params,
            init_params: Arc::new(init_params),
            receiver: req_receiver,
            sender: res_sender,
            server_sender,
        };
        tokio::spawn(client.run());

        let send = LSPClientSender { sender: req_sender };
        let read = LSPClientReader {
            receiver: res_receiver,
        };
        Ok((send, read))
    }
}

#[derive(Debug)]
pub struct LSPClientReader {
    receiver: Receiver<Response>,
}

impl LSPClientReader {
    pub async fn recv(&mut self) -> Option<Response> {
        self.receiver.recv().await
    }
}

#[derive(Debug)]
pub struct LSPClientSender {
    sender: Sender<Request>,
}

impl LSPClientSender {
    // TODO error?
    pub fn send(&mut self, req: Request) {
        let _ = self.sender.blocking_send(req);
    }
}

struct LSPClient {
    params: Arc<LSPClientParams>,
    init_params: Arc<lsp_types::InitializeResult>,
    receiver: Receiver<Request>,
    sender: Sender<Response>,

    server_sender: Sender<ServerRequest>,
}

impl LSPClient {
    pub async fn run(mut self) -> Result<()> {
        while let Some(req) = self.receiver.recv().await {
            let handler = Handler {
                params: self.params.clone(),
                init_params: self.init_params.clone(),
                server: self.server_sender.clone(),
                response: self.sender.clone(),
            };
            tokio::spawn(async move {
                let _ = handler.run(req).await;
            });
        }
        Ok(())
    }
}

struct Handler {
    params: Arc<LSPClientParams>,
    init_params: Arc<lsp_types::InitializeResult>,
    server: Sender<ServerRequest>,
    response: Sender<Response>,
}

impl Handler {
    pub async fn run(mut self, req: Request) -> Result<()> {
        match req {
            Request::DidOpen { path, buf } => self.did_open_document(path, buf).await?,
            Request::Hover {
                path,
                position,
                buf,
            } => self.hover(path, buf, position).await?,
            Request::GotoDefinition {
                path,
                position,
                buf,
            } => self.goto_definition(path, buf, position).await?,
            Request::DidChange { path, buf, changes } => {
                self.did_change_document(path, buf, changes).await?
            }
            Request::Complete {
                path,
                buf,
                position,
            } => self.complete(path, buf, position).await?,
        }

        Ok(())
    }

    async fn did_change_document(
        &mut self,
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        changes: Vec<Change>,
    ) -> Result<()> {
        let content_changes = changes
            .into_iter()
            .map(|change| lsp_types::TextDocumentContentChangeEvent {
                range: Some(lsp_types::Range {
                    start: change.start.to_position(&buf, &self.position_encoding()),
                    end: change.end.to_position(&buf, &self.position_encoding()),
                }),
                text: change.text,
                range_length: None,
            })
            .collect();

        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier {
                uri: path_to_uri(&path),
                // TODO
                version: 1,
            },
            content_changes,
        };

        let response = self
            .notify::<lsp_types::notification::DidChangeTextDocument>(&params)
            .await?;
        Ok(())
    }

    async fn complete(
        &mut self,
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        position: Position,
    ) -> Result<()> {
        let position = position.to_position(&buf, &self.position_encoding());
        let params = lsp_types::CompletionParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: path_to_uri(&path),
                },
                position,
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp_types::PartialResultParams {
                partial_result_token: None,
            },
            context: Some(lsp_types::CompletionContext {
                trigger_kind: lsp_types::CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        };

        let response = self
            .request::<lsp_types::request::Completion>(&params)
            .await?;
        let response = response.ok_or(anyhow!("No completion response"))?;
        match response {
            lsp_types::CompletionResponse::Array(_) => todo!(),
            lsp_types::CompletionResponse::List(list) => {
                for item in list.items {
                    // log::info!("Item: {:?}", item);
                    if let Some(edit) = item.text_edit {
                        match edit {
                            lsp_types::CompletionTextEdit::Edit(_) => todo!(),
                            lsp_types::CompletionTextEdit::InsertAndReplace(edit) => {
                                log::info!("Item: {}", edit.new_text);
                            }
                        }
                    }
                }
            }
        }

        // TODO response

        Ok(())
    }

    async fn goto_definition(
        &mut self,
        path: PathBuf,
        buf: ReadOnlyPieceTree,
        position: Position,
    ) -> Result<()> {
        let position = position.to_position(&buf, &self.position_encoding());
        let params = lsp_types::GotoDefinitionParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: path_to_uri(&path),
                },
                position,
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp_types::PartialResultParams {
                partial_result_token: None,
            },
        };

        let response = self
            .request::<lsp_types::request::GotoDefinition>(&params)
            .await?;
        let response = response.ok_or(anyhow!("No goto definition response"))?;

        let path;
        let position;
        match response {
            lsp_types::GotoDefinitionResponse::Scalar(_) => todo!("Scalar goto def"),
            lsp_types::GotoDefinitionResponse::Array(locations) => {
                let location = locations
                    .get(0)
                    .ok_or(anyhow!("Goto definition response found no locations"))?;
                path = PathBuf::from(location.uri.path().as_str());
                position = location.range.start;
            }
            lsp_types::GotoDefinitionResponse::Link(_) => todo!("Link gotodef"),
        }

        let _ = self
            .response
            .send(Response::Request(RequestResult::GotoDefinition {
                path,
                position: Position::LSP {
                    position,
                    encoding: self.position_encoding(),
                },
            }))
            .await;

        Ok(())
    }

    async fn hover(&mut self, path: PathBuf, buf: ReadOnlyPieceTree, pos: Position) -> Result<()> {
        let position = pos.to_position(&buf, &self.position_encoding());
        let params = lsp_types::HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: path_to_uri(&path),
                },
                position,
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let response = self
            .request::<lsp_types::request::HoverRequest>(&params)
            .await?;
        let response = response.ok_or(anyhow!("No hover response"))?;

        let text;
        match response.contents {
            lsp_types::HoverContents::Scalar(_) => todo!(),
            lsp_types::HoverContents::Array(_) => todo!(),
            lsp_types::HoverContents::Markup(cont) => match cont.kind {
                lsp_types::MarkupKind::PlainText => {
                    text = cont.value;
                }
                lsp_types::MarkupKind::Markdown => todo!(),
            },
        }

        let _ = self
            .response
            .send(Response::Request(RequestResult::Hover {
                text,
                position: pos,
            }))
            .await;

        Ok(())
    }

    async fn notify<R: lsp_types::notification::Notification>(
        &mut self,
        params: &R::Params,
    ) -> Result<()> {
        let json = JsonNotification::new(R::METHOD, &params);
        let msg = ServerRequest::Notification { json };
        self.server.send(msg).await?;

        Ok(())
    }

    async fn did_open_document(&mut self, path: PathBuf, buf: ReadOnlyPieceTree) -> Result<()> {
        let text = String::from(&buf);
        let params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: path_to_uri(&path),
                language_id: self.filetype().to_string(),
                version: 0,
                text,
            },
        };

        self.notify::<lsp_types::notification::DidOpenTextDocument>(&params)
            .await?;

        Ok(())
    }

    async fn request<R: lsp_types::request::Request>(
        &mut self,
        params: &R::Params,
    ) -> Result<R::Result> {
        let id = Self::next_request_id();
        let json = JsonRequest::new(R::METHOD, &params, id);
        let (tx, mut rx) = channel(1);
        let msg = ServerRequest::Request { json, answer: tx };
        self.server.send(msg).await?;

        let response = rx
            .recv()
            .await
            .ok_or(anyhow!("No answer to request {}", R::METHOD))??;

        let result = serde_json::from_value(response)?;

        Ok(result)
    }

    fn next_request_id() -> u32 {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }

    pub fn filetype(&self) -> &str {
        &self.params.filetype
    }

    fn position_encoding(&self) -> lsp_types::PositionEncodingKind {
        self.init_params
            .capabilities
            .position_encoding
            .clone()
            .unwrap_or(lsp_types::PositionEncodingKind::UTF16)
    }
}
