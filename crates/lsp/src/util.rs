use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use sanedit_buffer::{utf8::EndOfLine, PieceTreeSlice};
use sanedit_core::Severity;

pub fn path_to_uri(path: &Path) -> lsp_types::Uri {
    let uri = format!("file://{}", path.to_string_lossy());
    lsp_types::Uri::from_str(&uri).unwrap()
}

#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub file_edits: Vec<FileEdit>,
}

impl From<lsp_types::WorkspaceEdit> for WorkspaceEdit {
    fn from(value: lsp_types::WorkspaceEdit) -> Self {
        let mut file_edits = vec![];

        let lsp_types::WorkspaceEdit {
            changes,
            document_changes,
            change_annotations: _,
        } = value;

        if let Some(changes) = changes {
            for (uri, edits) in changes.into_iter() {
                let path = PathBuf::from(uri.path().as_str());
                let edits = edits.into_iter().map(TextEdit::from).collect();
                file_edits.push(FileEdit { path, edits });
            }
        }

        if let Some(changes) = document_changes {
            match changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for edit in edits {
                        file_edits.push(edit.into());
                    }
                }
                lsp_types::DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            lsp_types::DocumentChangeOperation::Op(_op) => todo!(),
                            lsp_types::DocumentChangeOperation::Edit(edit) => {
                                file_edits.push(edit.into())
                            }
                        }
                    }
                }
            }
        }

        WorkspaceEdit { file_edits }
    }
}

#[derive(Debug, Clone)]
pub struct FileEdit {
    pub path: PathBuf,
    pub edits: Vec<TextEdit>,
}

impl From<lsp_types::TextDocumentEdit> for FileEdit {
    fn from(value: lsp_types::TextDocumentEdit) -> Self {
        let path = PathBuf::from(value.text_document.uri.path().as_str());
        let edits = value
            .edits
            .into_iter()
            .map(|edit| match edit {
                lsp_types::OneOf::Left(a) => a,
                lsp_types::OneOf::Right(b) => b.text_edit,
            })
            .map(TextEdit::from)
            .collect();

        FileEdit { path, edits }
    }
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: PositionRange,
    pub text: String,
}

impl From<lsp_types::TextEdit> for TextEdit {
    fn from(value: lsp_types::TextEdit) -> Self {
        TextEdit {
            range: value.range.into(),
            text: value.new_text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextDiagnostic {
    pub line: u64,
    pub severity: Severity,
    pub description: String,
    pub range: PositionRange,
}

impl From<lsp_types::Diagnostic> for TextDiagnostic {
    fn from(diag: lsp_types::Diagnostic) -> Self {
        let severity = diag
            .severity
            .map(|sev| match sev {
                lsp_types::DiagnosticSeverity::ERROR => Severity::Error,
                lsp_types::DiagnosticSeverity::INFORMATION => Severity::Info,
                lsp_types::DiagnosticSeverity::WARNING => Severity::Warn,
                lsp_types::DiagnosticSeverity::HINT => Severity::Hint,
                _ => unreachable!(),
            })
            .unwrap_or(Severity::Hint);

        TextDiagnostic {
            line: diag.range.start.line as u64,
            severity,
            description: diag.message,
            range: diag.range.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub text: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub snippet: bool,
}

#[derive(Debug, Clone)]
pub struct CodeAction {
    pub(crate) action: lsp_types::CodeAction,
}

impl CodeAction {
    pub fn workspace_edit(self) -> Option<WorkspaceEdit> {
        let old = self.action.edit?;
        Some(WorkspaceEdit::from(old))
    }

    pub fn name(&self) -> &str {
        &self.action.title
    }

    pub fn is_resolved(&self) -> bool {
        self.action.edit.is_some()
    }
}

#[derive(Debug, Clone)]
pub enum PositionEncoding {
    UTF8,
    UTF16,
    UTF32,
}

impl From<lsp_types::PositionEncodingKind> for PositionEncoding {
    fn from(kind: lsp_types::PositionEncodingKind) -> Self {
        if kind == lsp_types::PositionEncodingKind::UTF8 {
            PositionEncoding::UTF8
        } else if kind == lsp_types::PositionEncodingKind::UTF16 {
            PositionEncoding::UTF16
        } else if kind == lsp_types::PositionEncodingKind::UTF32 {
            PositionEncoding::UTF32
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionRange {
    pub start: Position,
    pub end: Position,
}

impl From<lsp_types::Range> for PositionRange {
    fn from(range: lsp_types::Range) -> Self {
        PositionRange {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}

impl From<PositionRange> for lsp_types::Range {
    fn from(value: PositionRange) -> Self {
        lsp_types::Range {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pos: lsp_types::Position,
}

impl From<lsp_types::Position> for Position {
    fn from(value: lsp_types::Position) -> Self {
        Position { pos: value }
    }
}

impl From<Position> for lsp_types::Position {
    fn from(value: Position) -> Self {
        value.pos
    }
}

impl Position {
    pub fn new(mut offset: u64, slice: &PieceTreeSlice, enc: &PositionEncoding) -> Self {
        let (mut row, line) = slice.line_at(offset);
        offset -= line.start();

        let mut chars = line.chars();
        let mut col = 0u32;

        while let Some((start, _, ch)) = chars.next() {
            if start >= offset {
                break;
            }
            let len = match enc {
                PositionEncoding::UTF8 => ch.len_utf8(),
                PositionEncoding::UTF16 => ch.len_utf16(),
                PositionEncoding::UTF32 => 1,
            };

            col += len as u32;

            if EndOfLine::is_eol_char(ch) {
                row += 1;
                col = 0;
            }
        }

        Position {
            pos: lsp_types::Position {
                line: row as u32,
                character: col,
            },
        }
    }

    pub fn to_offset(&self, slice: &PieceTreeSlice, enc: &PositionEncoding) -> u64 {
        let lsp_types::Position { line, character } = self.pos;
        let Some(pos) = slice.pos_at_line(line as u64) else {
            return slice.len();
        };
        let mut chars = slice.chars_at(pos);
        let mut col = 0u32;

        while let Some((start, _, ch)) = chars.next() {
            if col >= character {
                return start;
            }
            let len = match enc {
                PositionEncoding::UTF8 => ch.len_utf8(),
                PositionEncoding::UTF16 => ch.len_utf16(),
                PositionEncoding::UTF32 => 1,
            };

            col += len as u32;
        }

        slice.len()
    }

    pub(crate) fn as_lsp(&self) -> lsp_types::Position {
        self.pos
    }
}
