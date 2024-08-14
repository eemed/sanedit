mod reader;
mod writer;

use std::sync::Arc;
use std::{path::PathBuf, process::Stdio};

use crate::util::path_to_uri;
use crate::Operation;

use self::reader::Reader;
use self::writer::{LSPWrite, Writer};

use anyhow::{anyhow, Result};
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Notify;
use tokio::{
    io::BufReader,
    process::{Child, Command},
};

/// Common struct between LSP writer and reader halves
struct Common {
    params: LSPClientParams,
    _process: Child,
}

impl Common {
    pub fn root_uri(&self) -> lsp_types::Uri {
        path_to_uri(&self.params.root)
    }

    pub fn filetype(&self) -> &str {
        &self.params.filetype
    }
}

/// a struct to put all the parameters
pub struct LSPClientParams {
    pub run_command: String,
    pub run_args: Vec<String>,
    pub root: PathBuf,
    pub filetype: String,
}

impl LSPClientParams {
    pub async fn spawn(self) -> Result<LSPClient> {
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
        let (tx, rx) = channel(256);
        let initialized = Arc::new(Notify::new());

        let common = Arc::new(Common {
            params: self,
            _process: cmd,
        });
        let writer = Writer {
            common: common.clone(),
            stdin,
            receiver: rx,
            initialized: initialized.clone(),
        };
        let reader = Reader {
            common: common.clone(),
            stdout,
            stderr,
            sender: tx.clone(),
        };

        // TODO handle failures and shutdown
        tokio::spawn(writer.run());
        tokio::spawn(reader.run());

        // Wait for initialization
        initialized.notified().await;

        let client = LSPClient { sender: tx };
        Ok(client)
    }
}

#[derive(Debug)]
pub struct LSPClient {
    sender: Sender<LSPWrite>,
}

impl LSPClient {
    // TODO error?
    pub fn send(&mut self, op: Operation) {
        let _ = self.sender.blocking_send(LSPWrite::Op(op));
    }
}
