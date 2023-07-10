use std::collections::HashSet;

use crate::PieceTreeSlice;

lazy_static! {
    static ref EOL_BYTES: HashSet<Vec<u8>> = {
        use EndOfLine::*;

        let mut set = HashSet::new();
        set.insert(AsRef::<[u8]>::as_ref(&LF).into());
        set.insert(AsRef::<[u8]>::as_ref(&VT).into());
        set.insert(AsRef::<[u8]>::as_ref(&FF).into());
        set.insert(AsRef::<[u8]>::as_ref(&CR).into());
        set.insert(AsRef::<[u8]>::as_ref(&CRLF).into());
        set.insert(AsRef::<[u8]>::as_ref(&NEL).into());
        set.insert(AsRef::<[u8]>::as_ref(&LS).into());
        set.insert(AsRef::<[u8]>::as_ref(&PS).into());
        set
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndOfLine {
    LF,   // LF: Line Feed, U+000A (UTF-8 in hex: 0A)
    VT,   // VT: Vertical Tab, U+000B (UTF-8 in hex: 0B)
    FF,   // FF: Form Feed, U+000C (UTF-8 in hex: 0C)
    CR,   // CR: Carriage Return, U+000D (UTF-8 in hex: 0D)
    CRLF, // CR+LF: CR (U+000D) followed by LF (U+000A) (UTF-8 in hex: 0D 0A)
    NEL,  // NEL: Next Line, U+0085 (UTF-8 in hex: C2 85)
    LS,   // LS: Line Separator, U+2028 (UTF-8 in hex: E2 80 A8)
    PS,   // PS: Paragraph Separator, U+2029 (UTF-8 in hex: E2 80 A9)
}

impl EndOfLine {
    pub fn is_eol<B: AsRef<[u8]>>(bytes: B) -> bool {
        let bytes = bytes.as_ref();
        EOL_BYTES.contains(bytes)
    }

    pub fn is_slice_eol(slice: &PieceTreeSlice) -> bool {
        EOL_BYTES.iter().any(|eol| slice == eol)
    }

    pub fn len(&self) -> usize {
        let string: &str = self.as_ref();
        string.len()
    }
}

impl AsRef<str> for EndOfLine {
    fn as_ref(&self) -> &str {
        use EndOfLine::*;

        match self {
            LF => "\u{000A}",
            VT => "\u{000B}",
            FF => "\u{000C}",
            CR => "\u{000D}",
            CRLF => "\u{000D}\u{000A}",
            NEL => "\u{0085}",
            LS => "\u{2028}",
            PS => "\u{2029}",
        }
    }
}

impl AsRef<[u8]> for EndOfLine {
    fn as_ref(&self) -> &[u8] {
        let string: &str = self.as_ref();
        string.as_bytes()
    }
}

impl Default for EndOfLine {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        const DEFAULT_EOL: EndOfLine = EndOfLine::CRLF;

        #[cfg(not(target_os = "windows"))]
        const DEFAULT_EOL: EndOfLine = EndOfLine::LF;

        DEFAULT_EOL
    }
}
