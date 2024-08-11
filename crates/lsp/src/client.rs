use std::path::Path;
use std::{path::PathBuf, process::Stdio};

use super::capabilities::client_capabilities;
use super::jsonrpc::{Notification, Response};
use anyhow::{anyhow, Result};
use lsp_types::TextDocumentPositionParams;
use sanedit_buffer::ReadOnlyPieceTree;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

use crate::jsonrpc::Request;
use crate::util::path_to_uri;
use crate::Operation;

/// Just a struct to put all the parameters
pub struct LSPStartParams {
    pub run_command: String,
    pub run_args: Vec<String>,
    pub root: PathBuf,
    pub filetype: String,
}

pub struct LSPClient {
    root: PathBuf,
    filetype: String,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
    _process: Child,
}

impl LSPClient {
    /// Start a new LSP process
    pub fn new(ctx: LSPStartParams) -> Result<LSPClient> {
        // Spawn server
        let mut cmd = Command::new(&ctx.run_command)
            .args(&*ctx.run_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = cmd.stdin.take().ok_or(anyhow!("Failed to take stdin"))?;
        let stdout = cmd.stdout.take().ok_or(anyhow!("Failed to take stdout"))?;
        let stderr = cmd.stderr.take().ok_or(anyhow!("Failed to take stderr"))?;

        Ok(LSPClient {
            root: ctx.root,
            filetype: ctx.filetype,
            stdin,
            stdout: BufReader::new(stdout),
            stderr: BufReader::new(stderr),
            _process: cmd,
        })
    }

    /// Initialize capabilities to the LSP
    pub async fn start(&mut self) -> Result<()> {
        self.initialize().await?;
        let _response = self.read_response().await?;

        self.initialized().await?;
        let _response = self.read_response().await?;

        Ok(())
    }

    pub async fn log_strerr(&mut self) {
        let mut buf = String::new();
        while let Ok(n) = self.stderr.read_line(&mut buf).await {
            if n == 0 {
                break;
            }

            log::info!("{buf}");
            buf.clear();
        }
    }

    async fn initialize(&mut self) -> Result<()> {
        let params = lsp_types::InitializeParams {
            process_id: std::process::id().into(),
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: client_capabilities(),
            trace: None,
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: path_to_uri(&self.root),
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
        let content = Request::new("initialize", &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    async fn initialized(&mut self) -> Result<()> {
        let params = lsp_types::InitializedParams {};
        let content = Notification::new("initialized", &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    async fn hover(&mut self, path: PathBuf, offset: usize) -> Result<()> {
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

        let content = Request::new("textDocument/hover", &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    async fn did_open_document(&mut self, path: PathBuf, buf: ReadOnlyPieceTree) -> Result<()> {
        let text = String::from(&buf);
        let params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: path_to_uri(&path),
                language_id: self.filetype.clone(),
                version: 0,
                text,
            },
        };

        let content = Notification::new("textDocument/didOpen", &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<Response> {
        let response = Response::read_from(&mut self.stdout).await?;
        Ok(response)
    }

    pub async fn operate(&mut self, op: Operation) -> Result<()> {
        log::info!("Operate: {op:?}");
        match op {
            Operation::DidOpen { path, buf } => self.did_open_document(path, buf).await?,
            Operation::Hover { path, offset } => self.hover(path, offset).await?,
        }

        Ok(())
    }
}
