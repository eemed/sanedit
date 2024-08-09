mod capabilities;

use std::{ffi::OsStr, path::PathBuf, process::Stdio, str::FromStr};

use anyhow::{anyhow, Result};
use capabilities::client_capabilities;
use lsp_types::{ClientCapabilities, Uri};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
};

pub struct LSPServer {
    stdin: ChildStdin,
}

impl LSPServer {
    pub fn new(ctx: LSPContext) {}

    pub fn run(&mut self) {}
}

pub struct LSPContext {
    pub run_command: String,
    pub run_args: Vec<String>,
    pub root: PathBuf,
}

impl LSPContext {
    pub async fn spawn(&mut self) -> Result<()> {
        let root = self.root.to_string_lossy().to_string();
        let mut cmd = Command::new(&self.run_command)
            .args(&*self.run_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        log::info!("Spawned");
        let mut stdin = cmd.stdin.take().ok_or(anyhow!("Failed to take stdin"))?;
        let mut stdout = cmd.stdout.take().ok_or(anyhow!("Failed to take stdout"))?;
        log::info!("Taken");

        let init = lsp_types::InitializeParams {
            process_id: std::process::id().into(),
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: client_capabilities(),
            trace: None,
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: lsp_types::Uri::from_str(&root)?,
                name: root,
            }]),
            client_info: Some(lsp_types::ClientInfo {
                name: String::from("sanedit"),
                version: None,
            }),
            locale: None,
            work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
        };

        log::info!("Writing...");
        let json = serde_json::to_string(&init)?;
        // TODO format into a message
        log::info!("Send: {json}");
        stdin.write_all(json.as_bytes()).await?;
        log::info!("Written");

        log::info!("Reading");
        let mut out = BufReader::new(stdout);

        let mut buf = String::new();
        while out.read_line(&mut buf).await.is_ok() {
            log::info!("LSP: {buf:?}");
            buf.clear();
        }

        // stdin.write_all();

        // if let Ok(output) = child.wait_with_output().await {
        //     log::info!(
        //         "Ran '{}', stdout: {}, stderr: {}",
        //         command,
        //         std::str::from_utf8(&output.stdout).unwrap(),
        //         std::str::from_utf8(&output.stderr).unwrap(),
        //     )
        // }
        // }
        Ok(())
    }
}
