use std::{collections::BTreeMap, path::PathBuf};

use crate::util::{
    CodeAction, CompletionItem, FileEdit, Position, PositionRange, Symbol, TextDiagnostic, WorkspaceEdit
};

#[derive(Debug, Clone)]
pub enum Response {
    Request { id: u32, result: RequestResult },
    Notification(NotificationResult),
}

#[derive(Debug, Clone)]
pub enum NotificationResult {
    Diagnostics {
        path: PathBuf,
        version: Option<i32>,
        diagnostics: Vec<TextDiagnostic>,
    },
}

// TODO should clear out lsp_types::* from here
// to provide a simple interface to the editor.
#[derive(Debug, Clone)]
pub enum RequestResult {
    Hover {
        text: String,
        position: Position,
    },
    GotoDefinition {
        path: PathBuf,
        position: Position,
    },
    Complete {
        path: PathBuf,
        position: Position,
        results: Vec<CompletionItem>,
    },
    References {
        references: BTreeMap<PathBuf, Vec<PositionRange>>,
    },
    CodeAction {
        actions: Vec<CodeAction>,
    },
    ResolvedAction {
        action: CodeAction,
    },
    Rename {
        workspace_edit: WorkspaceEdit,
    },
    Format {
        edit: FileEdit,
    },
    Diagnostics {
        path: PathBuf,
        diagnostics: Vec<TextDiagnostic>,
    },
    Error {
        msg: String,
    },
    Symbols {
        symbols: Vec<Symbol>,
    },
}
