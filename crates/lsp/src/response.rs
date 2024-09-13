use std::{collections::BTreeMap, path::PathBuf};

use sanedit_buffer::{PieceTreeSlice, PieceTreeView};
use sanedit_core::{BufferRange, Diagnostic};

#[derive(Debug, Clone)]
pub enum Response {
    Request { id: u32, result: RequestResult },
    Notification(NotificationResult),
}

// TODO should clear out lsp_types::* from here
// to provide a simple interface to the editor.
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
pub enum NotificationResult {
    Diagnostics {
        path: PathBuf,
        version: Option<i32>,
        diagnostics: Vec<LSPRange<Diagnostic>>,
    },
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reference {
    pub start: lsp_types::Position,
    pub end: lsp_types::Position,
}

/// Wraps a structure which references a lsp range.
/// This needs to be decoded before the inner T can be accessed
#[derive(Debug, Clone)]
pub struct LSPRange<T> {
    pub(crate) t: T,
    pub(crate) range: lsp_types::Range,
}

/// Wraps a structure which references a lsp position.
/// This needs to be decoded before the inner T can be accessed
#[derive(Debug, Clone)]
pub struct LSPPosition<T> {
    pub(crate) t: T,
    pub(crate) range: lsp_types::Position,
}

impl LSPRange<Diagnostic> {
    fn decode(
        &mut self,
        slice: &PieceTreeSlice,
        enc: &lsp_types::PositionEncodingKind,
    ) -> Diagnostic {
        let range = range_to_buffer_range(slice, self.range, enc);
        Diagnostic::new(*self.t.severity(), range, self.t.description())
    }
}

pub(crate) fn offset_to_position(
    slice: &PieceTreeSlice,
    mut offset: u64,
    kind: &lsp_types::PositionEncodingKind,
) -> lsp_types::Position {
    let (row, line) = slice.line_at(offset);
    offset -= line.start();

    let mut chars = line.chars();
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if start >= offset {
            break;
        }
        let len = if *kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if *kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if *kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    lsp_types::Position {
        line: row as u32,
        character: col,
    }
}

pub(crate) fn range_to_buffer_range(
    slice: &PieceTreeSlice,
    range: lsp_types::Range,
    kind: &lsp_types::PositionEncodingKind,
) -> BufferRange {
    let start = position_to_offset(slice, range.start, kind);
    let end = position_to_offset(slice, range.end, kind);
    start..end
}

pub(crate) fn position_to_offset(
    slice: &PieceTreeSlice,
    position: lsp_types::Position,
    kind: &lsp_types::PositionEncodingKind,
) -> u64 {
    let lsp_types::Position { line, character } = position;
    let pos = slice.pos_at_line(line as u64);
    let mut chars = slice.chars_at(pos);
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if col >= character {
            return start;
        }
        let len = if *kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if *kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if *kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    unreachable!("Position not found")
}
