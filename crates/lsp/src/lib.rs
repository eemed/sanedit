mod capabilities;
mod jsonrpc;

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    str::FromStr,
};

use anyhow::{anyhow, Result};
use capabilities::client_capabilities;
use jsonrpc::{Notification, Response};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
};

use crate::jsonrpc::Request;

/// Just a struct to put all the parameters
pub struct LSPStartParams {
    pub run_command: String,
    pub run_args: Vec<String>,
    pub root: PathBuf,
}

pub struct LSPClient {
    root: PathBuf,
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
            stdin,
            stdout: BufReader::new(stdout),
            stderr: BufReader::new(stderr),
            _process: cmd,
        })
    }

    /// Initialize capabilities to the LSP
    pub async fn start(&mut self) -> Result<()> {
        self.initialize().await?;

        let response = self.read_response().await?;
        log::info!("Init response: {response:?}");

        self.initialized().await?;
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
        let init = lsp_types::InitializeParams {
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
        let method = "initialize";

        let content = Request::new(method, Some(&init));
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    async fn initialized(&mut self) -> Result<()> {
        let init = lsp_types::InitializedParams {};
        let method = "initialized";

        let content = Notification::new(method, Some(&init));
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<Response> {
        let response = Response::read_from(&mut self.stdout).await?;
        Ok(response)
    }
}

fn path_to_uri(path: &Path) -> lsp_types::Uri {
    let uri = format!("file://{}", path.to_string_lossy());
    lsp_types::Uri::from_str(&uri).unwrap()
}
