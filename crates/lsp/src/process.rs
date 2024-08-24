use std::collections::BTreeMap;
use std::sync::Arc;

use crate::capabilities::client_capabilities;
use crate::jsonrpc::{read_from, JsonNotification, JsonRequest, JsonResponse};
use crate::util::path_to_uri;
use crate::LSPClientParams;

use anyhow::{anyhow, bail, Result};
use lsp_types::notification::Notification;
use lsp_types::request::Request as _;
use sanedit_utils::either::Either;
use serde_json::Value;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tokio::{io::BufReader, process::Child};

#[derive(Debug)]
pub(crate) enum ServerRequest {
    Request {
        json: JsonRequest,
        answer: Sender<Result<Value>>,
    },
    Notification {
        json: JsonNotification,
    },
}

pub(crate) struct ProcessHandler {
    pub(crate) params: Arc<LSPClientParams>,
    pub(crate) _process: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) stdout: BufReader<ChildStdout>,
    pub(crate) stderr: BufReader<ChildStderr>,

    pub(crate) receiver: Receiver<ServerRequest>,
    pub(crate) initialized: Option<oneshot::Sender<Result<lsp_types::InitializeResult>>>,

    pub(crate) in_flight: BTreeMap<u32, Sender<Result<Value>>>,
}

impl ProcessHandler {
    pub async fn run(mut self) -> Result<()> {
        let init_result = self.initialize().await;
        let ok = init_result.is_ok();
        let init = std::mem::take(&mut self.initialized).unwrap();
        let _ = init.send(init_result);

        if !ok {
            bail!("Initialization failed");
        }

        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    let msg = msg.ok_or(anyhow!("LSP sender is closed"))?;
                    match msg {
                        ServerRequest::Request { json, answer } => self.handle_request(json, answer).await?,
                        ServerRequest::Notification { json } => self.handle_notification(json).await?,
                    }
                }
                json = read_from(&mut self.stdout) => {
                    match json? {
                        Either::Right(notification) => {
                            // log::info!("{notification:?}");
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
        // log::info!("Response: {response:?}");
        if response.result.is_none() && response.error.is_none() {
            return Ok(());
        }

        let sender = self.in_flight.remove(&response.id).ok_or(anyhow!(
            "Got a response to non existent request {}",
            response.id
        ))?;

        let result = response.result.ok_or(anyhow!("{:?}", response.error));
        let _ = sender.send(result).await;
        Ok(())
    }

    async fn initialize(&mut self) -> Result<lsp_types::InitializeResult> {
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
        let response = self.read_response().await?;
        let value = response
            .result
            .ok_or(anyhow!("Server responded with empty initialize result"))?;
        let result = serde_json::from_value::<lsp_types::InitializeResult>(value)?;

        // Send initialized notification
        let params = lsp_types::InitializedParams {};
        let content = JsonNotification::new(lsp_types::notification::Initialized::METHOD, &params);
        content.write_to(&mut self.stdin).await?;

        Ok(result)
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
