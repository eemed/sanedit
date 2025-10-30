use std::{cmp::min, collections::HashSet, sync::OnceLock};

use crate::{PieceTree, PieceTreeSlice};

use super::{next_eol, prev_eol};

fn eol_bytes() -> &'static HashSet<Vec<u8>> {
    static SET: OnceLock<HashSet<Vec<u8>>> = OnceLock::new();
    SET.get_or_init(|| {
        use EndOfLine::*;

        let mut set = HashSet::new();
        set.insert(AsRef::<[u8]>::as_ref(&Lf).into());
        set.insert(AsRef::<[u8]>::as_ref(&Vt).into());
        set.insert(AsRef::<[u8]>::as_ref(&Ff).into());
        set.insert(AsRef::<[u8]>::as_ref(&Cr).into());
        set.insert(AsRef::<[u8]>::as_ref(&Crlf).into());
        set.insert(AsRef::<[u8]>::as_ref(&Nel).into());
        set.insert(AsRef::<[u8]>::as_ref(&Ls).into());
        set.insert(AsRef::<[u8]>::as_ref(&Ps).into());
        set
    })
}

impl PieceTreeSlice {
    /// Whether this slice is a single EOL
    pub fn is_eol(&self) -> bool {
        EndOfLine::is_slice_eol(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EndOfLine {
    Lf,   // LF: Line Feed, U+000A (UTF-8 in hex: 0A)
    Vt,   // VT: Vertical Tab, U+000B (UTF-8 in hex: 0B)
    Ff,   // FF: Form Feed, U+000C (UTF-8 in hex: 0C)
    Cr,   // CR: Carriage Return, U+000D (UTF-8 in hex: 0D)
    Crlf, // CR+LF: CR (U+000D) followed by LF (U+000A) (UTF-8 in hex: 0D 0A)
    Nel,  // NEL: Next Line, U+0085 (UTF-8 in hex: C2 85)
    Ls,   // LS: Line Separator, U+2028 (UTF-8 in hex: E2 80 A8)
    Ps,   // PS: Paragraph Separator, U+2029 (UTF-8 in hex: E2 80 A9)
}

impl EndOfLine {
    /// Maximum eol length in bytes
    pub const MAX_EOL_LEN: usize = 3;

    pub fn all() -> &'static [EndOfLine] {
        use EndOfLine::*;
        &[Lf, Vt, Ff, Cr, Crlf, Nel, Ls, Ps]
    }

    pub fn strip_eol<'a>(slice: &PieceTreeSlice) -> PieceTreeSlice {
        let start = slice.len().saturating_sub(Self::MAX_EOL_LEN as u64);
        let end = slice.len();
        let potential = slice.slice(start..end);
        let mut bytes = potential.bytes_at(potential.len());
        if let Some(eol) = prev_eol(&mut bytes) {
            slice.slice(..slice.len() - eol.eol.len())
        } else {
            slice.clone()
        }
    }

    pub fn is_eol_char(ch: char) -> bool {
        let mut buf = [0; 4];
        let s = ch.encode_utf8(&mut buf);
        Self::is_eol(s.as_bytes())
    }

    pub fn has_eol<B: AsRef<[u8]>>(bytes: B) -> bool {
        let pt = PieceTree::from_reader(std::io::Cursor::new(bytes.as_ref())).unwrap();
        let mut bytes = pt.bytes();
        next_eol(&mut bytes).is_some()
    }

    pub fn is_eol<B: AsRef<[u8]>>(bytes: B) -> bool {
        let bytes = bytes.as_ref();
        eol_bytes().contains(bytes)
    }

    pub fn is_eol_prefix<B: AsRef<[u8]>>(bytes: B) -> Option<EndOfLine> {
        let bytes = bytes.as_ref();
        let top = min(Self::MAX_EOL_LEN, bytes.len());
        let pt = PieceTree::from_reader(std::io::Cursor::new(&bytes[..top])).unwrap();
        let mut bytes = pt.bytes();
        let mat = next_eol(&mut bytes)?;
        if mat.range.start != 0 {
            return None;
        }
        Some(mat.eol)
    }

    pub fn is_byte_eol(byte: u8) -> bool {
        matches!(byte, 0x0a..=0x0d)
    }

    pub fn is_slice_eol(slice: &PieceTreeSlice) -> bool {
        if slice.len() > Self::MAX_EOL_LEN as u64 {
            return false;
        }

        let mut buf = [0u8; Self::MAX_EOL_LEN];
        let mut bytes = slice.bytes();
        let mut i = 0;
        while let Some(byte) = bytes.next() {
            buf[i] = byte;
            i += 1;
        }

        Self::is_eol(&buf[..i])
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    pub fn len(&self) -> u64 {
        let string: &str = self.as_ref();
        string.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<str> for EndOfLine {
    fn as_ref(&self) -> &str {
        use EndOfLine::*;

        match self {
            Lf => "\u{000A}",
            Vt => "\u{000B}",
            Ff => "\u{000C}",
            Cr => "\u{000D}",
            Crlf => "\u{000D}\u{000A}",
            Nel => "\u{0085}",
            Ls => "\u{2028}",
            Ps => "\u{2029}",
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
        const DEFAULT_EOL: EndOfLine = EndOfLine::Crlf;

        #[cfg(not(target_os = "windows"))]
        const DEFAULT_EOL: EndOfLine = EndOfLine::Lf;

        DEFAULT_EOL
    }
}
