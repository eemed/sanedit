use crate::PieceTreeSlice;

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
        let _bytes = bytes.as_ref();
        todo!()
    }

    pub fn is_slice_eol(_slice: &PieceTreeSlice) -> bool {
        todo!()
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
