use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Request {
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
}

#[derive(Debug, Clone)]
pub struct Change {
    pub start: lsp_types::Position,
    pub end: lsp_types::Position,
    pub text: String,
}
