use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::{path::PathBuf, process::Stdio};

use crate::capabilities::client_capabilities;
use crate::jsonrpc::{read_from, JsonNotification, JsonRequest, JsonResponse};
use crate::util::path_to_uri;
use crate::{Request, RequestResult};

use anyhow::{anyhow, bail, Result};
use sanedit_buffer::ReadOnlyPieceTree;
use sanedit_utils::either::Either;
use serde::Serialize;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Notify;
use tokio::{
    io::BufReader,
    process::{Child, Command},
};

/// a struct to put all the parameters
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
        let (wsend, wrecv) = channel(256);
        let (osend, orecv) = channel(256);
        let initialized = Arc::new(Notify::new());

        let client = LSPClient {
            params: self,
            _process: cmd,
            stdin,
            stdout,
            stderr,
            receiver: wrecv,
            sender: osend,
            initialized: initialized.clone(),
            in_flight: BTreeMap::default(),
        };

        tokio::spawn(client.run());

        // Wait for initialization
        initialized.notified().await;

        let send = LSPClientSender { sender: wsend };
        let read = LSPClientReader { receiver: orecv };
        Ok((send, read))
    }
}

#[derive(Debug)]
enum Method {
    Initialize,
    Initialized,
    Hover,
}

impl Method {
    fn as_str(&self) -> &str {
        match self {
            Method::Hover => "textDocument/hover",
            Method::Initialize => "initialize",
            Method::Initialized => "initialized",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    Request(RequestResult),
    // Notification(),
}

struct LSPClient {
    params: LSPClientParams,
    _process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,

    receiver: Receiver<Request>,
    sender: Sender<Response>,
    initialized: Arc<Notify>,

    in_flight: BTreeMap<u32, Method>,
}

impl LSPClient {
    pub async fn run(mut self) -> Result<()> {
        let init_result = self.initialize().await;
        self.initialized.notify_one();
        init_result?;

        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    let req = msg.ok_or(anyhow!("LSP sender is closed"))?;
                    self.do_request(req).await?;
                }
                json = read_from(&mut self.stdout) => {
                    match json? {
                        Either::Right(notification) => {
                            // log::info!("{notification:?}");
                        }
                        Either::Left(response) => {
                            let method = self.in_flight.remove(&response.id)
                                .ok_or(anyhow!("Got a response to non existent request {}", response.id))?;

                            log::info!("{method:?}: {response:?}");
                        }
                    }
                }
            };
        }

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
        let content = JsonRequest::new(
            Method::Initialize.as_str(),
            &params,
            Self::next_request_id(),
        );
        content.write_to(&mut self.stdin).await?;

        // Read server response
        let _response = self.read_response().await?;

        // Send initialized notification
        let params = lsp_types::InitializedParams {};
        let content = JsonNotification::new(Method::Initialized.as_str(), &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    pub async fn do_request(&mut self, req: Request) -> Result<()> {
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

        self.request(Method::Hover, &params).await
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

        let content = JsonNotification::new("textDocument/didOpen", &params);
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

    async fn request<T>(&mut self, method: Method, params: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let id = Self::next_request_id();
        let content = JsonRequest::new(method.as_str(), &params, id);
        content.write_to(&mut self.stdin).await?;
        self.in_flight.insert(id, method);
        Ok(())
    }

    pub fn root_uri(&self) -> lsp_types::Uri {
        path_to_uri(&self.params.root)
    }

    pub fn filetype(&self) -> &str {
        &self.params.filetype
    }

    fn next_request_id() -> u32 {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
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
