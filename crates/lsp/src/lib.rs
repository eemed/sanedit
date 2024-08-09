mod capabilities;

use std::{ffi::OsStr, path::PathBuf, process::Stdio};

use anyhow::{anyhow, Result};
use capabilities::client_capabilities;
use lsp_types::{ClientCapabilities, Uri};
use tokio::{
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
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
    run_command: Box<OsStr>,
    run_args: Box<[Box<OsStr>]>,
    root: PathBuf,
}

impl LSPContext {
    async fn spawn(&mut self) -> Result<()> {
        let mut cmd = Command::new(&self.run_command)
            .args(&*self.run_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let stdin = cmd.stdin.take().ok_or(anyhow!("Failed to take stdin"))?;
        let stdout = cmd.stdout.take().ok_or(anyhow!("Failed to take stdout"))?;

        let init = lsp_types::InitializeParams {
            process_id: std::process::id().into(),
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: client_capabilities(),
            trace: todo!(),
            workspace_folders: todo!(),
            client_info: Some(lsp_types::ClientInfo {
                name: String::from("sanedit"),
                version: None,
            }),
            locale: None,
            work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
        };

        let json = serde_json::to_string(&init)?;
        stdin.write_all(json.as_bytes()).await?;

        let mut res = String::new();
        stdout.read_to_string(&mut res).await?;

        log::info!("RES: {res:?}");

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
