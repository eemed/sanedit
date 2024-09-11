use serde::{Deserialize, Serialize};

use sanedit_buffer::{utf8::prev_eol, Bytes, PieceTreeSlice};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum IndentKind {
    Space,
    Tab,
}

impl IndentKind {
    pub fn as_str(&self) -> &str {
        match self {
            IndentKind::Space => " ",
            IndentKind::Tab => "\t",
        }
    }

    pub fn as_byte(&self) -> u8 {
        match self {
            IndentKind::Space => b' ',
            IndentKind::Tab => b'\t',
        }
    }

    pub fn repeat(&self, n: usize) -> String {
        self.as_str().repeat(n)
    }
}

pub fn determine_indent(slice: &PieceTreeSlice) -> Option<(IndentKind, u8)> {
    let mut indents = vec![];
    let mut lines = slice.lines();
    while let Some(line) = lines.next() {
        let mut bytes = line.bytes();
        let indent = indent_from_bytes(&mut bytes);
        if let Some(indent) = indent {
            indents.push(indent);
        }
    }

    // Default
    if indents.is_empty() {
        return None;
    }

    let (spaces, tabs): (Vec<(IndentKind, u64)>, Vec<(IndentKind, u64)>) = indents
        .into_iter()
        .partition(|(kind, _)| *kind == IndentKind::Space);

    if spaces.len() >= tabs.len() {
        let min = spaces.iter().map(|(_, i)| i).min().unwrap();
        Some((
            IndentKind::Space,
            TryInto::try_into(*min).unwrap_or(u8::MAX),
        ))
    } else {
        let min = tabs.iter().map(|(_, i)| i).min().unwrap();
        Some((IndentKind::Tab, TryInto::try_into(*min).unwrap_or(u8::MAX)))
    }
}

fn indent_from_bytes(bytes: &mut Bytes) -> Option<(IndentKind, u64)> {
    use IndentKind::*;
    let kind = bytes
        .next()
        .map(|b| {
            if b == Tab.as_byte() {
                Some(Tab)
            } else if b == Space.as_byte() {
                Some(Space)
            } else {
                None
            }
        })
        .flatten()?;

    let mut n = 1;
    while let Some(b) = bytes.next() {
        if b == kind.as_byte() {
            n += 1;
        } else {
            break;
        }
    }

    Some((kind, n))
}

/// Calculate indentation level at a line where pos is at
pub fn indent_at_line(slice: &PieceTreeSlice, pos: u64) -> Option<(IndentKind, u64)> {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }

    indent_from_bytes(&mut bytes)
}

/// If pos is at indentation
pub fn is_indent_at_pos(slice: &PieceTreeSlice, pos: u64) -> bool {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }

    let start = bytes.pos();
    let Some((_, n)) = indent_from_bytes(&mut bytes) else {
        return false;
    };
    let end = start + n;

    if end == slice.len() {
        return true;
    }

    if !(start..=end).contains(&pos) {
        return false;
    }

    true
}
