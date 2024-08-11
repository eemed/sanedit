use anyhow::{bail, Result};
use sanedit_utils::either::Either;
use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStderr, ChildStdout},
    sync::mpsc::Sender,
};

use crate::jsonrpc::{read_from, Response};

use super::{writer::LSPWrite, Common};

pub struct Reader {
    pub(super) common: Arc<Common>,
    pub(super) stdout: BufReader<ChildStdout>,
    pub(super) stderr: BufReader<ChildStderr>,
    pub(super) sender: Sender<LSPWrite>,
}

impl Reader {
    pub async fn run(mut self) -> Result<()> {
        self.initialize().await?;

        // Process messages from LSP
        loop {
            let msg = read_from(&mut self.stdout).await?;
            match msg {
                Either::Right(notification) => {
                    log::info!("{notification:?}");
                }
                Either::Left(response) => {
                    log::info!("{response:?}");
                }
            }
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        if let Ok(response) = self.read_response().await {
            self.sender.send(LSPWrite::Initialized(response)).await?;
        }

        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<Response> {
        let response = read_from(&mut self.stdout).await?;
        if response.is_right() {
            bail!("Got notification instead of response")
        }

        Ok(response.take_left().unwrap())
    }

    async fn log_strerr(&mut self) {
        let mut buf = String::new();
        while let Ok(n) = self.stderr.read_line(&mut buf).await {
            if n == 0 {
                break;
            }

            log::info!("{buf}");
            buf.clear();
        }
    }
}
