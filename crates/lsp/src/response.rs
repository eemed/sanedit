use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Response {
    Request(RequestResult),
    // Notification(),
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
        references: Vec<Reference>,
    },
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    // pub description: Option<String>,
    // pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub path: PathBuf,
    pub start: lsp_types::Position,
    pub end: lsp_types::Position,
}
