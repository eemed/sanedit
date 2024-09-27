use std::collections::BTreeMap;
use std::sync::Arc;
use std::{path::PathBuf, process::Stdio};

use crate::jsonrpc::{JsonNotification, JsonRequest};
use crate::process::{ProcessHandler, ServerRequest};
use crate::request::{Notification, RequestKind, ToLSP};
use crate::response::{NotificationResult, Reference};
use crate::util::path_to_uri;
use crate::{Change, CompletionItem, LSPRange, Request, RequestResult, Response};

use anyhow::{anyhow, Result};
use lsp_types::notification::Notification as _;
use sanedit_core::{Diagnostic, Severity};
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
        let (server_notif_sender, server_notif_recv) = channel(256);
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
            _stderr: stderr,
            receiver: server_recv,
            notification_sender: server_notif_sender,
            initialized: Some(init_send),
            in_flight: BTreeMap::default(),
        };

        tokio::spawn(server.run());

        // Wait for initialization
        let init_params = Arc::new(init_recv.await??);

        let client = LSPClient {
            params,
            receiver: req_receiver,
            sender: res_sender,
            server_sender,
            server_notification_receiver: server_notif_recv,
        };
        tokio::spawn(client.run());

        let send = LSPClientSender {
            init_params,
            sender: req_sender,
        };
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
    init_params: Arc<lsp_types::InitializeResult>,
    sender: Sender<ToLSP>,
}

impl LSPClientSender {
    pub fn request(&mut self, req: Request) {
        let _ = self.sender.blocking_send(ToLSP::Request(req));
    }

    pub fn notify(&mut self, notif: Notification) {
        let _ = self.sender.blocking_send(ToLSP::Notification(notif));
    }

    pub fn init_params(&self) -> &lsp_types::InitializeResult {
        &self.init_params
    }
}

struct LSPClient {
    params: Arc<LSPClientParams>,
    receiver: Receiver<ToLSP>,
    sender: Sender<Response>,
    server_sender: Sender<ServerRequest>,
    server_notification_receiver: Receiver<JsonNotification>,
}

impl LSPClient {
    pub async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                req = self.receiver.recv() => {
                    let req = req.ok_or(anyhow!("Channel closed"))?;
                    let mut handler = Handler {
                        params: self.params.clone(),
                        server: self.server_sender.clone(),
                        response: self.sender.clone(),
                    };

                    tokio::spawn(async move {
                        let id = req.id();
                        if let Err(e) = handler.run(req).await {
                            log::error!("Failed handling request: {e}");

                            // Send error response if failed request
                            if let Some(id) = id {
                                let _ = handler
                                    .response
                                    .send(Response::Request {
                                        id,
                                        result: RequestResult::Error {
                                            msg: format!("{e}"),
                                        },
                                    })
                                    .await;
                            }
                        }
                    });
                }
                notif = self.server_notification_receiver.recv() => {
                    let notif = notif.ok_or(anyhow!("Notification channel closed"))?;
                    self.handle_notification(notif).await?;
                }
            }
        }
    }

    async fn handle_notification(&mut self, notif: JsonNotification) -> Result<()> {
        match notif.method.as_str() {
            lsp_types::notification::PublishDiagnostics::METHOD => {
                let params =
                    serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(notif.params)?;
                let path = PathBuf::from(params.uri.path().as_str());
                let diagnostics = NotificationResult::Diagnostics {
                    path,
                    version: params.version,
                    diagnostics: params
                        .diagnostics
                        .into_iter()
                        .map(to_core_diagnostic)
                        .collect(),
                };

                self.sender
                    .send(Response::Notification(diagnostics))
                    .await?;
            }
            lsp_types::notification::Progress::METHOD => {}
            lsp_types::notification::ShowMessage::METHOD => {}
            _ => {}
        }

        Ok(())
    }
}

fn to_core_diagnostic(diag: lsp_types::Diagnostic) -> LSPRange<Diagnostic> {
    let severity = diag
        .severity
        .map(|sev| match sev {
            lsp_types::DiagnosticSeverity::ERROR => Severity::Error,
            lsp_types::DiagnosticSeverity::INFORMATION => Severity::Info,
            lsp_types::DiagnosticSeverity::WARNING => Severity::Warn,
            lsp_types::DiagnosticSeverity::HINT => Severity::Hint,
            _ => unreachable!(),
        })
        .unwrap_or(Severity::Hint);

    let diagnostic = Diagnostic::new(severity, 0..0, &diag.message);

    LSPRange {
        t: diagnostic,
        range: diag.range,
    }
}

struct Handler {
    params: Arc<LSPClientParams>,
    server: Sender<ServerRequest>,
    response: Sender<Response>,
}

impl Handler {
    pub async fn run(&mut self, msg: ToLSP) -> Result<()> {
        match msg {
            ToLSP::Request(req) => match req.kind {
                RequestKind::Hover { path, position } => self.hover(req.id, path, position).await?,
                RequestKind::GotoDefinition { path, position } => {
                    self.goto_definition(req.id, path, position).await?
                }
                RequestKind::Complete { path, position } => {
                    self.complete(req.id, path, position).await?
                }
                RequestKind::References { path, position } => {
                    self.show_references(req.id, path, position).await?
                }
                RequestKind::CodeAction { path, position } => {
                    self.code_action(req.id, path, position).await?
                }
                RequestKind::CodeActionResolve { action } => {
                    self.code_action_resolve(req.id, action).await?
                }
                RequestKind::Rename {
                    path,
                    position,
                    new_name,
                } => self.rename(req.id, path, position, new_name).await?,
            },
            ToLSP::Notification(notif) => match notif {
                Notification::DidOpen {
                    path,
                    text,
                    version,
                } => self.did_open_document(path, text, version).await?,
                Notification::DidChange {
                    path,
                    version,
                    changes,
                } => self.did_change_document(path, changes, version).await?,
                Notification::DidClose { path } => self.did_close_document(path).await?,
            },
        }

        Ok(())
    }

    async fn rename(
        &mut self,
        id: u32,
        path: PathBuf,
        position: lsp_types::Position,
        new_name: String,
    ) -> Result<()> {
        let params = lsp_types::RenameParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: path_to_uri(&path),
                },
                position,
            },
            new_name,
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let response = self
            .request::<lsp_types::request::Rename>(id, &params)
            .await?;

        let result = response.ok_or(anyhow!("No rename response"))?;
        let _ = self
            .response
            .send(Response::Request {
                id,
                result: RequestResult::Rename { edit: result },
            })
            .await;

        Ok(())
    }

    async fn code_action_resolve(&mut self, id: u32, action: lsp_types::CodeAction) -> Result<()> {
        let response = self
            .request::<lsp_types::request::CodeActionResolveRequest>(id, &action)
            .await?;

        let _ = self
            .response
            .send(Response::Request {
                id,
                result: RequestResult::ResolvedAction { action: response },
            })
            .await;
        Ok(())
    }

    async fn code_action(
        &mut self,
        id: u32,
        path: PathBuf,
        position: lsp_types::Position,
    ) -> Result<()> {
        let params = lsp_types::CodeActionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: path_to_uri(&path),
            },
            range: lsp_types::Range {
                start: position,
                end: position,
            },
            context: lsp_types::CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: Some(lsp_types::CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp_types::PartialResultParams {
                partial_result_token: None,
            },
        };

        let response = self
            .request::<lsp_types::request::CodeActionRequest>(id, &params)
            .await?;
        let response = response.ok_or(anyhow!("No code action response"))?;
        let mut actions = vec![];

        for cmd in response {
            match cmd {
                lsp_types::CodeActionOrCommand::Command(_cmd) => todo!(),
                lsp_types::CodeActionOrCommand::CodeAction(action) => {
                    actions.push(action);
                }
            }
        }

        let _ = self
            .response
            .send(Response::Request {
                id,
                result: RequestResult::CodeAction { actions },
            })
            .await;

        Ok(())
    }

    async fn did_change_document(
        &mut self,
        path: PathBuf,
        changes: Vec<Change>,
        version: i32,
    ) -> Result<()> {
        let content_changes = changes
            .into_iter()
            .map(|change| lsp_types::TextDocumentContentChangeEvent {
                range: Some(lsp_types::Range {
                    start: change.start,
                    end: change.end,
                }),
                text: change.text,
                range_length: None,
            })
            .collect();

        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier {
                uri: path_to_uri(&path),
                version,
            },
            content_changes,
        };

        self.notify::<lsp_types::notification::DidChangeTextDocument>(&params)
            .await
    }

    async fn complete(
        &mut self,
        id: u32,
        path: PathBuf,
        position: lsp_types::Position,
    ) -> Result<()> {
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
            .request::<lsp_types::request::Completion>(id, &params)
            .await?;
        let response = response.ok_or(anyhow!("No completion response"))?;

        let mut results = vec![];
        match response {
            lsp_types::CompletionResponse::Array(_) => todo!(),
            lsp_types::CompletionResponse::List(list) => {
                for item in list.items {
                    match item.text_edit {
                        Some(lsp_types::CompletionTextEdit::Edit(_)) => todo!(),
                        Some(lsp_types::CompletionTextEdit::InsertAndReplace(edit)) => {
                            results.push(CompletionItem {
                                name: edit.new_text,
                                description: item.kind.map(|kind| {
                                    let mut desc = format!("{kind:?}");
                                    desc.make_ascii_lowercase();
                                    desc
                                }),
                                documentation: item.documentation.map(|doc| match doc {
                                    lsp_types::Documentation::String(doc) => doc,
                                    lsp_types::Documentation::MarkupContent(mdoc) => mdoc.value,
                                }),
                            });
                        }
                        None => {}
                    }
                }
            }
        }

        let _ = self
            .response
            .send(Response::Request {
                id,
                result: RequestResult::Complete {
                    path,
                    position,
                    results,
                },
            })
            .await;

        Ok(())
    }

    async fn goto_definition(
        &mut self,
        id: u32,
        path: PathBuf,
        position: lsp_types::Position,
    ) -> Result<()> {
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
            .request::<lsp_types::request::GotoDefinition>(id, &params)
            .await?;
        let response = response.ok_or(anyhow!("No goto definition response"))?;

        let path;
        let position;
        match response {
            lsp_types::GotoDefinitionResponse::Scalar(_) => todo!("Scalar goto def"),
            lsp_types::GotoDefinitionResponse::Array(locations) => {
                let location = locations
                    .first()
                    .ok_or(anyhow!("Goto definition response found no locations"))?;
                path = PathBuf::from(location.uri.path().as_str());
                position = location.range.start;
            }
            lsp_types::GotoDefinitionResponse::Link(_) => todo!("Link gotodef"),
        }

        let _ = self
            .response
            .send(Response::Request {
                id,
                result: RequestResult::GotoDefinition { path, position },
            })
            .await;

        Ok(())
    }

    async fn hover(&mut self, id: u32, path: PathBuf, position: lsp_types::Position) -> Result<()> {
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
            .request::<lsp_types::request::HoverRequest>(id, &params)
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
                lsp_types::MarkupKind::Markdown => {
                    text = cont.value;
                }
            },
        }

        self.response
            .send(Response::Request {
                id,
                result: RequestResult::Hover { text, position },
            })
            .await?;

        Ok(())
    }

    async fn did_open_document(&mut self, path: PathBuf, text: String, version: i32) -> Result<()> {
        let params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: path_to_uri(&path),
                language_id: self.filetype().to_string(),
                version,
                text,
            },
        };

        self.notify::<lsp_types::notification::DidOpenTextDocument>(&params)
            .await?;

        Ok(())
    }

    async fn did_close_document(&mut self, path: PathBuf) -> Result<()> {
        let params = lsp_types::DidCloseTextDocumentParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: path_to_uri(&path),
            },
        };

        self.notify::<lsp_types::notification::DidCloseTextDocument>(&params)
            .await?;

        Ok(())
    }

    async fn show_references(
        &mut self,
        id: u32,
        path: PathBuf,
        position: lsp_types::Position,
    ) -> Result<()> {
        let params = lsp_types::ReferenceParams {
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
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let response = self
            .request::<lsp_types::request::References>(id, &params)
            .await?;
        let locations = response.ok_or(anyhow!("No references response"))?;
        let mut references = BTreeMap::new();
        for loc in locations {
            let re = Reference {
                start: loc.range.start,
                end: loc.range.end,
            };
            let path = PathBuf::from(loc.uri.path().as_str());
            let entry = references.entry(path);
            let value: &mut Vec<Reference> = entry.or_default();
            value.push(re);
        }

        self.response
            .send(Response::Request {
                id,
                result: RequestResult::References { references },
            })
            .await?;

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

    async fn request<R: lsp_types::request::Request>(
        &mut self,
        id: u32,
        params: &R::Params,
    ) -> Result<R::Result> {
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

    pub fn filetype(&self) -> &str {
        &self.params.filetype
    }
}
