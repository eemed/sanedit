use std::path::PathBuf;

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
        changes: Vec<Change>,
        version: i32,
    },
    DidClose {
        path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct Request {
    pub id: u32,
    pub kind: RequestKind,
}

#[derive(Debug, Clone)]
pub enum RequestKind {
    Hover {
        path: PathBuf,
        position: lsp_types::Position,
    },
    GotoDefinition {
        path: PathBuf,
        position: lsp_types::Position,
    },
    Complete {
        path: PathBuf,
        position: lsp_types::Position,
    },
    References {
        path: PathBuf,
        position: lsp_types::Position,
    },
    CodeAction {
        path: PathBuf,
        position: lsp_types::Position,
    },
    CodeActionResolve {
        action: lsp_types::CodeAction,
    },
    Rename {
        path: PathBuf,
        position: lsp_types::Position,
        new_name: String,
    },
}

#[derive(Debug, Clone)]
pub struct Change {
    pub start: lsp_types::Position,
    pub end: lsp_types::Position,
    pub text: String,
}
