use std::path::PathBuf;

use sanedit_core::IndentKind;
use sanedit_utils::either::Either;

use crate::util::{CodeAction, Position, TextEdit};

#[derive(Debug, Clone)]
pub(crate) enum ToLSP {
    Request(Request),
    Notification(Notification),
}

impl ToLSP {
    pub fn id(&self) -> Option<u32> {
        match self {
            ToLSP::Request(req) => Some(req.id),
            ToLSP::Notification(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Notification {
    DidOpen {
        path: PathBuf,
        text: String,
        version: i32,
    },
    DidChange {
        path: PathBuf,
        /// Either partial or full change
        changes: Either<Vec<TextEdit>, String>,
        version: i32,
    },
    DidClose {
        path: PathBuf,
    },
    WillSave {
        path: PathBuf,
    },
    DidSave {
        path: PathBuf,
        text: Option<String>,
    },
}

impl Notification {
    /// Determines if the notification is supported by the LSP.
    /// May also modify the notification to make it supported.
    pub fn is_supported(&mut self, init: &lsp_types::InitializeResult) -> bool {
        // TODO determine if server supports this request
        true
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub id: u32,
    pub kind: RequestKind,
}

impl Request {
    /// Determines if the request is supported by the LSP.
    /// May also modify the request to make it supported.
    pub fn is_supported(&mut self, init: &lsp_types::InitializeResult) -> bool {
        // TODO determine if server supports this request
        true
    }
}

#[derive(Debug, Clone)]
pub enum RequestKind {
    Hover {
        path: PathBuf,
        position: Position,
    },
    GotoDefinition {
        path: PathBuf,
        position: Position,
    },
    Complete {
        path: PathBuf,
        position: Position,
    },
    References {
        path: PathBuf,
        position: Position,
    },
    CodeAction {
        path: PathBuf,
        position: Position,
    },
    CodeActionResolve {
        action: CodeAction,
    },
    Rename {
        path: PathBuf,
        position: Position,
        new_name: String,
    },
    Format {
        path: PathBuf,
        indent_kind: IndentKind,
        indent_amount: u32,
    },
    PullDiagnostics {
        path: PathBuf,
    },
}
