use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    str::FromStr,
};

use sanedit_buffer::{utf8::EndOfLine, PieceTreeSlice};
use sanedit_core::{BufferRange, Range, Severity};
use sanedit_utils::either::Either;
use strum_macros::AsRefStr;

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl From<lsp_types::InsertReplaceEdit> for TextEdit {
    fn from(value: lsp_types::InsertReplaceEdit) -> Self {
        TextEdit {
            range: value.replace.into(),
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
            description: diag.message.trim().into(),
            range: diag.range.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, Hash)]
pub enum CompletionItemKind {
    EnumMember,
    Field,
    Constant,
    Property,
    Text,
    Method,
    Function,
    Constructor,
    Variable,
    Class,
    Interface,
    Module,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl From<lsp_types::CompletionItemKind> for CompletionItemKind {
    fn from(value: lsp_types::CompletionItemKind) -> Self {
        match value {
            lsp_types::CompletionItemKind::TEXT => CompletionItemKind::Text,
            lsp_types::CompletionItemKind::METHOD => CompletionItemKind::Method,
            lsp_types::CompletionItemKind::FUNCTION => CompletionItemKind::Function,
            lsp_types::CompletionItemKind::CONSTRUCTOR => CompletionItemKind::Constructor,
            lsp_types::CompletionItemKind::FIELD => CompletionItemKind::Field,
            lsp_types::CompletionItemKind::VARIABLE => CompletionItemKind::Variable,
            lsp_types::CompletionItemKind::CLASS => CompletionItemKind::Class,
            lsp_types::CompletionItemKind::INTERFACE => CompletionItemKind::Interface,
            lsp_types::CompletionItemKind::MODULE => CompletionItemKind::Module,
            lsp_types::CompletionItemKind::PROPERTY => CompletionItemKind::Property,
            lsp_types::CompletionItemKind::UNIT => CompletionItemKind::Unit,
            lsp_types::CompletionItemKind::VALUE => CompletionItemKind::Value,
            lsp_types::CompletionItemKind::ENUM => CompletionItemKind::Enum,
            lsp_types::CompletionItemKind::KEYWORD => CompletionItemKind::Keyword,
            lsp_types::CompletionItemKind::SNIPPET => CompletionItemKind::Snippet,
            lsp_types::CompletionItemKind::COLOR => CompletionItemKind::Color,
            lsp_types::CompletionItemKind::FILE => CompletionItemKind::File,
            lsp_types::CompletionItemKind::REFERENCE => CompletionItemKind::Reference,
            lsp_types::CompletionItemKind::FOLDER => CompletionItemKind::Folder,
            lsp_types::CompletionItemKind::ENUM_MEMBER => CompletionItemKind::EnumMember,
            lsp_types::CompletionItemKind::CONSTANT => CompletionItemKind::Constant,
            lsp_types::CompletionItemKind::STRUCT => CompletionItemKind::Struct,
            lsp_types::CompletionItemKind::EVENT => CompletionItemKind::Event,
            lsp_types::CompletionItemKind::OPERATOR => CompletionItemKind::Operator,
            lsp_types::CompletionItemKind::TYPE_PARAMETER => CompletionItemKind::TypeParameter,
            _ => CompletionItemKind::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompletionItem {
    pub kind: CompletionItemKind,

    /// Text to insert
    pub text: String,

    /// Text to use to filter or text if not present
    pub filter: Option<String>,

    pub detail: Option<String>,

    /// Overrides text to insert
    pub edit: Option<TextEdit>,

    /// Additional text edits, for example import at the top
    pub additional_edits: Vec<TextEdit>,

    /// If text format is snippet, may contain for example $0
    pub is_snippet: bool,
}

impl PartialOrd for CompletionItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompletionItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.kind.cmp(&other.kind) {
            Ordering::Equal => match self.text.len().cmp(&other.text.len()) {
                Ordering::Equal => self.text.cmp(&other.text),
                len_order => len_order,
            },
            kind_order => kind_order,
        }
    }
}

impl CompletionItem {
    pub fn filter_text(&self) -> &str {
        self.filter.as_ref().unwrap_or(&self.text)
    }

    pub fn insert_text(&self) -> Either<&str, &TextEdit> {
        if let Some(edit) = &self.edit {
            Either::Right(edit)
        } else {
            Either::Left(self.text.as_str())
        }
    }

    /// Description to show in completion
    pub fn description(&self) -> &str {
        if let Some(ref detail) = self.detail {
            detail.as_ref()
        } else {
            self.kind.as_ref()
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodeAction {
    pub(crate) action: lsp_types::CodeAction,
}

impl CodeAction {
    pub fn workspace_edit(&self) -> Option<WorkspaceEdit> {
        let old = self.action.edit.clone()?;
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PositionRange {
    pub start: Position,
    pub end: Position,
}

impl PositionRange {
    pub fn to_buffer_range(&self, slice: &PieceTreeSlice, enc: &PositionEncoding) -> BufferRange {
        let start = self.start.to_offset(slice, enc);
        let end = if self.start == self.end {
            start
        } else {
            self.end.to_offset(slice, enc)
        };

        Range::new(start, end)
    }
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

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub position: Position,
}

impl From<lsp_types::SymbolInformation> for Symbol {
    fn from(symbol: lsp_types::SymbolInformation) -> Self {
        let range = symbol.location.range;
        let name = symbol.name;
        let kind: SymbolKind = symbol.kind.into();

        Symbol {
            name,
            kind,
            position: range.start.into(),
        }
    }
}

impl From<lsp_types::DocumentSymbol> for Symbol {
    fn from(symbol: lsp_types::DocumentSymbol) -> Self {
        let range = symbol.range;
        let name = symbol.name;
        let kind: SymbolKind = symbol.kind.into();

        Symbol {
            name,
            kind,
            position: range.start.into(),
        }
    }
}

impl From<lsp_types::WorkspaceSymbol> for Symbol {
    fn from(symbol: lsp_types::WorkspaceSymbol) -> Self {
        let position = match symbol.location {
            lsp_types::OneOf::Left(left) => left.range.start.into(),
            lsp_types::OneOf::Right(_) => unreachable!(),
        };
        let name = symbol.name;
        let kind: SymbolKind = symbol.kind.into();

        Symbol {
            name,
            kind,
            position,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, Hash)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl From<lsp_types::SymbolKind> for SymbolKind {
    fn from(value: lsp_types::SymbolKind) -> Self {
        match value {
            lsp_types::SymbolKind::FILE => SymbolKind::File,
            lsp_types::SymbolKind::MODULE => SymbolKind::Module,
            lsp_types::SymbolKind::NAMESPACE => SymbolKind::Namespace,
            lsp_types::SymbolKind::PACKAGE => SymbolKind::Package,
            lsp_types::SymbolKind::CLASS => SymbolKind::Class,
            lsp_types::SymbolKind::METHOD => SymbolKind::Method,
            lsp_types::SymbolKind::PROPERTY => SymbolKind::Property,
            lsp_types::SymbolKind::FIELD => SymbolKind::Field,
            lsp_types::SymbolKind::CONSTRUCTOR => SymbolKind::Constructor,
            lsp_types::SymbolKind::ENUM => SymbolKind::Enum,
            lsp_types::SymbolKind::INTERFACE => SymbolKind::Interface,
            lsp_types::SymbolKind::FUNCTION => SymbolKind::Function,
            lsp_types::SymbolKind::VARIABLE => SymbolKind::Variable,
            lsp_types::SymbolKind::CONSTANT => SymbolKind::Constant,
            lsp_types::SymbolKind::STRING => SymbolKind::String,
            lsp_types::SymbolKind::NUMBER => SymbolKind::Number,
            lsp_types::SymbolKind::BOOLEAN => SymbolKind::Boolean,
            lsp_types::SymbolKind::ARRAY => SymbolKind::Array,
            lsp_types::SymbolKind::OBJECT => SymbolKind::Object,
            lsp_types::SymbolKind::KEY => SymbolKind::Key,
            lsp_types::SymbolKind::NULL => SymbolKind::Null,
            lsp_types::SymbolKind::ENUM_MEMBER => SymbolKind::EnumMember,
            lsp_types::SymbolKind::STRUCT => SymbolKind::Struct,
            lsp_types::SymbolKind::EVENT => SymbolKind::Event,
            lsp_types::SymbolKind::OPERATOR => SymbolKind::Operator,
            lsp_types::SymbolKind::TYPE_PARAMETER => SymbolKind::TypeParameter,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Signatures {
    /// All signatures, functions may be overloaded
    pub signatures: Vec<Signature>,
    pub active_signature: Option<usize>,
}

impl Signatures {
    pub fn active_signature(&self) -> Option<&Signature> {
        let pos = self.active_signature?;
        self.signatures.get(pos)
    }
}

impl From<lsp_types::SignatureHelp> for Signatures {
    fn from(help: lsp_types::SignatureHelp) -> Self {
        let mut signatures: Vec<Signature> =
            help.signatures.into_iter().map(Signature::from).collect();
        let active_signature = help.active_signature.map(|n| n as usize);
        if let Some(active) = active_signature {
            let signature = &mut signatures[active];
            if signature.active_parameter.is_none() {
                signature.active_parameter = help.active_parameter.map(|n| n as usize);
            }
        }
        Signatures {
            signatures,
            active_signature,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub name: String,
    pub params: Vec<SignatureParameter>,
    pub active_parameter: Option<usize>,
}

impl Signature {
    pub fn active_parameter(&self) -> Option<&SignatureParameter> {
        let pos = self.active_parameter?;
        self.params.get(pos)
    }
}

impl From<lsp_types::SignatureInformation> for Signature {
    fn from(info: lsp_types::SignatureInformation) -> Self {
        let params = info
            .parameters
            .unwrap_or(vec![])
            .into_iter()
            .map(|param| SignatureParameter::new(param, &info.label))
            .collect();

        let active_parameter = info.active_parameter.map(|n| n as usize);

        Signature {
            name: info.label,
            params,
            active_parameter,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignatureParameter {
    pub name: String,
}

impl SignatureParameter {
    pub fn new(param: lsp_types::ParameterInformation, label: &str) -> SignatureParameter {
        let name = match param.label {
            lsp_types::ParameterLabel::Simple(name) => name,
            lsp_types::ParameterLabel::LabelOffsets(offsets) => {
                let start = offsets[0] as usize;
                let end = offsets[1] as usize;
                label[start..end].into()
            }
        };

        SignatureParameter { name }
    }
}
