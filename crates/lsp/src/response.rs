use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone)]
pub enum Response {
    Request { id: u32, result: RequestResult }, // Notification(),
}

#[derive(Debug, Clone)]
pub enum RequestResult {
    Hover {
        text: String,
        position: lsp_types::Position,
    },
    GotoDefinition {
        path: PathBuf,
        position: lsp_types::Position,
    },
    Complete {
        path: PathBuf,
        position: lsp_types::Position,
        results: Vec<CompletionItem>,
    },
    References {
        references: BTreeMap<PathBuf, Vec<Reference>>,
    },
    CodeAction {
        actions: Vec<lsp_types::CodeAction>,
    },
    ResolvedAction {
        action: lsp_types::CodeAction,
    },
    Rename {
        edit: lsp_types::WorkspaceEdit,
    },
    Error {
        msg: String,
    },
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    // pub description: Option<String>,
    // pub documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reference {
    pub start: lsp_types::Position,
    pub end: lsp_types::Position,
}
