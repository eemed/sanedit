use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::{path::PathBuf, process::Stdio};

use crate::capabilities::client_capabilities;
use crate::jsonrpc::{read_from, JsonNotification, JsonRequest, JsonResponse};
use crate::util::path_to_uri;
use crate::{Request, RequestResult};

use anyhow::{anyhow, bail, Result};
use lsp_types::notification::Notification;
use lsp_types::request::Request as _;
use sanedit_buffer::ReadOnlyPieceTree;
use sanedit_utils::either::Either;
use serde::Serialize;
use serde_json::Value;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Notify;
use tokio::{
    io::BufReader,
    process::{Child, Command},
};

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
        let server = Server {
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

#[derive(Debug, Clone)]
pub enum Response {
    Request(RequestResult),
    // Notification(),
}

struct Server {
    params: Arc<LSPClientParams>,
    _process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,

    receiver: Receiver<ServerRequest>,
    initialized: Arc<Notify>,

    in_flight: BTreeMap<u32, Sender<Result<Value>>>,
}

impl Server {
    pub async fn run(mut self) -> Result<()> {
        let init_result = self.initialize().await;
        self.initialized.notify_one();
        init_result?;

        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    let msg = msg.ok_or(anyhow!("LSP sender is closed"))?;
                    log::info!("Receive: {:?}", msg);
                    match msg {
                        ServerRequest::Request { json, answer } => self.handle_request(json, answer).await?,
                        ServerRequest::Notification { json } => self.handle_notification(json).await?,
                    }
                }
                json = read_from(&mut self.stdout) => {
                    log::info!("Read: {json:?}");
                    match json? {
                        Either::Right(notification) => {
                            log::info!("{notification:?}");
                        }
                        Either::Left(response) => self.handle_response(response).await?,
                    }
                }
            };
        }

        Ok(())
    }

    async fn handle_request(
        &mut self,
        json: JsonRequest,
        answer: Sender<Result<Value>>,
    ) -> Result<()> {
        let id = json.id();
        json.write_to(&mut self.stdin).await?;
        self.in_flight.insert(id, answer);
        Ok(())
    }

    async fn handle_notification(&mut self, json: JsonNotification) -> Result<()> {
        json.write_to(&mut self.stdin).await?;
        Ok(())
    }

    async fn handle_response(&mut self, response: JsonResponse) -> Result<()> {
        let sender = self.in_flight.remove(&response.id).ok_or(anyhow!(
            "Got a response to non existent request {}",
            response.id
        ))?;
        log::info!("Response: {response:?}");

        let result = response.result.ok_or(anyhow!("{:?}", response.error));
        let _ = sender.send(result).await;
        Ok(())
    }

    async fn initialize(&mut self) -> Result<()> {
        // Send initialize request
        let params = lsp_types::InitializeParams {
            process_id: std::process::id().into(),
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: client_capabilities(),
            trace: None,
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: self.root_uri(),
                name: "root".into(),
            }]),
            // workspace_folders: None,
            client_info: Some(lsp_types::ClientInfo {
                name: String::from("sanedit"),
                version: None,
            }),
            locale: None,
            work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
        };
        let content = JsonRequest::new(lsp_types::request::Initialize::METHOD, &params, 0);
        content.write_to(&mut self.stdin).await?;

        // Read server response
        let _response = self.read_response().await?;

        // Send initialized notification
        let params = lsp_types::InitializedParams {};
        let content = JsonNotification::new(lsp_types::notification::Initialized::METHOD, &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<JsonResponse> {
        let response = read_from(&mut self.stdout).await?;
        if response.is_right() {
            bail!("Got notification instead of response")
        }

        Ok(response.take_left().unwrap())
    }

    pub fn root_uri(&self) -> lsp_types::Uri {
        path_to_uri(&self.params.root)
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

#[derive(Debug)]
enum ServerRequest {
    Request {
        json: JsonRequest,
        answer: Sender<Result<Value>>,
    },
    Notification {
        json: JsonNotification,
    },
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
