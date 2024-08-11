use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use sanedit_buffer::ReadOnlyPieceTree;
use tokio::process::ChildStdin;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Notify;

use crate::jsonrpc::Response;
use crate::util::path_to_uri;
use crate::Operation;
use crate::{
    capabilities::client_capabilities,
    jsonrpc::{Notification, Request},
};

use super::Common;

#[derive(Clone)]
pub(crate) enum LSPWrite {
    Op(Operation),
    Initialized(Response),
}

pub(crate) struct Writer {
    pub(super) common: Arc<Common>,
    pub(super) stdin: ChildStdin,
    pub(super) receiver: Receiver<LSPWrite>,
    pub(super) initialized: Arc<Notify>,
}

impl Writer {
    pub async fn run(mut self) -> Result<()> {
        self.initialize().await?;

        while let Some(write) = self.receiver.recv().await {
            match write {
                LSPWrite::Op(op) => self.operate(op).await?,
                LSPWrite::Initialized(res) => self.initialized(res).await?,
            }
        }

        Ok(())
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
                uri: self.common.root_uri(),
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

    async fn initialized(&mut self, res: Response) -> Result<()> {
        let params = lsp_types::InitializedParams {};
        let content = Notification::new("initialized", &params);
        content.write_to(&mut self.stdin).await?;

        // Notify initialization is done
        self.initialized.notify_one();

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

        let content = Request::new("textDocument/hover", &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    async fn did_open_document(&mut self, path: PathBuf, buf: ReadOnlyPieceTree) -> Result<()> {
        let text = String::from(&buf);
        let params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: path_to_uri(&path),
                language_id: self.common.filetype().to_string(),
                version: 0,
                text,
            },
        };

        let content = Notification::new("textDocument/didOpen", &params);
        content.write_to(&mut self.stdin).await?;

        Ok(())
    }

    pub async fn operate(&mut self, op: Operation) -> Result<()> {
        log::info!("Operate: {op:?}");
        match op {
            Operation::DidOpen { path, buf } => self.did_open_document(path, buf).await?,
            Operation::Hover { path, offset, buf } => self.hover(path, buf, offset).await?,
        }

        Ok(())
    }
}
