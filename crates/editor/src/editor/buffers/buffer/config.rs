use serde::{Deserialize, Serialize};

use sanedit_buffer::utf8::EndOfLine;

use sanedit_core::IndentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(remote = "EndOfLine")]
enum EndOfLineDef {
    LF,
    VT,
    FF,
    CR,
    CRLF,
    NEL,
    LS,
    PS,
}

#[derive(Debug, Clone, Serialize, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct BufferConfig {
    /// Default EOL, overridden if detect_eol is set
    /// Available options:
    /// LF: Line Feed, U+000A (UTF-8 in hex: 0A)
    /// VT: Vertical Tab, U+000B (UTF-8 in hex: 0B)
    /// FF: Form Feed, U+000C (UTF-8 in hex: 0C)
    /// CR: Carriage Return, U+000D (UTF-8 in hex: 0D)
    /// CRLF: CR (U+000D) followed by LF (U+000A) (UTF-8 in hex: 0D 0A)
    /// NEL: Next Line, U+0085 (UTF-8 in hex: C2 85)
    /// LS: Line Separator, U+2028 (UTF-8 in hex: E2 80 A8)
    /// PS: Paragraph Separator, U+2029 (UTF-8 in hex: E2 80 A9)
    #[serde(with = "EndOfLineDef")]
    pub(crate) eol: EndOfLine,

    /// How many spaces for a tab character, note that tab does not always have
    /// width `tab_width` because tabs are used to align stuff, so it
    /// is "elastic" calculated tabstop - (col % tabstop).
    pub(crate) tabstop: u8,

    /// Indent options, overridden if detect_indent is set
    /// Available options:
    /// Space: use spaces
    /// Tab: use tabs
    pub(crate) indent_kind: IndentKind,

    /// How many indent characters a single indent should be
    pub(crate) indent_amount: u8,
}

impl Default for BufferConfig {
    fn default() -> Self {
        BufferConfig {
            eol: EndOfLine::default(),
            tabstop: 8,
            indent_kind: IndentKind::Space,
            indent_amount: 4,
        }
    }
}
