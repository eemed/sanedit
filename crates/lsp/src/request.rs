use std::path::PathBuf;

use sanedit_core::IndentKind;

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
        changes: Vec<TextEdit>,
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
}
