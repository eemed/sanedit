use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::{path::PathBuf, process::Stdio};

use crate::jsonrpc::{JsonNotification, JsonRequest};
use crate::process::{ProcessHandler, ServerRequest};
use crate::util::path_to_uri;
use crate::{Request, Response};

use anyhow::{anyhow, Result};
use sanedit_buffer::ReadOnlyPieceTree;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Notify;
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
        let initialized = Arc::new(Notify::new());

        let params = Arc::new(self);
        let server = ProcessHandler {
            params: params.clone(),
            _process: cmd,
            stdin,
            stdout,
            stderr,
            receiver: server_recv,
            initialized: initialized.clone(),
            in_flight: BTreeMap::default(),
        };

        tokio::spawn(server.run());

        // Wait for initialization
        initialized.notified().await;

        let client = LSPClient {
            params,
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
    receiver: Receiver<Request>,
    sender: Sender<Response>,

    server_sender: Sender<ServerRequest>,
}

impl LSPClient {
    pub async fn run(mut self) -> Result<()> {
        while let Some(req) = self.receiver.recv().await {
            let handler = Handler {
                params: self.params.clone(),
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
    server: Sender<ServerRequest>,
    response: Sender<Response>,
}

impl Handler {
    pub async fn run(mut self, req: Request) -> Result<()> {
        log::info!("do_request: {req:?}");
        match req {
            Request::DidOpen { path, buf } => self.did_open_document(path, buf).await?,
            Request::Hover { path, offset, buf } => self.hover(path, buf, offset).await?,
        }

        Ok(())
    }

    async fn hover(&mut self, path: PathBuf, buf: ReadOnlyPieceTree, offset: usize) -> Result<()> {
        let params = lsp_types::HoverParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: path_to_uri(&path),
                },
                position: lsp_types::Position {
                    line: 1,
                    character: 5,
                },
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let response = self
            .request::<lsp_types::request::HoverRequest>(&params)
            .await?;

        log::info!("Hover response: {:?}", response);

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

        // let content = JsonNotification::new("textDocument/didOpen", &params);
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
}
