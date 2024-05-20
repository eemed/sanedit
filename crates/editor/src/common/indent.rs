use sanedit_buffer::{
    utf8::{prev_eol, EndOfLine},
    Bytes, PieceTree, PieceTreeSlice,
};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
#[repr(u8)]
pub(crate) enum IndentKind {
    Space = b' ',
    Tab = b'\t',
}

impl IndentKind {
    pub fn as_str(&self) -> &str {
        match self {
            IndentKind::Space => " ",
            IndentKind::Tab => "\t",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub(crate) struct Indent {
    pub(crate) n: usize,
    pub(crate) kind: IndentKind,
}

impl Indent {
    pub fn get(&self, level: usize) -> String {
        self.kind.as_str().repeat(level * self.n)
    }

    pub fn to_string(&self) -> String {
        self.get(1)
    }

    pub fn determine(slice: &PieceTreeSlice) -> Indent {
        let mut indents = vec![];
        let mut lines = slice.lines();
        while let Some(line) = lines.next() {
            let mut bytes = line.bytes();
            let indent = indent_from_bytes(&mut bytes);
            if indent.n != 0 {
                indents.push(indent);
            }
        }

        // Default
        if indents.is_empty() {
            return Indent::default();
        }

        let (spaces, tabs): (Vec<Indent>, Vec<Indent>) = indents
            .into_iter()
            .partition(|indent| indent.kind == IndentKind::Space);

        if spaces.len() >= tabs.len() {
            let min = spaces.iter().map(|i| i.n).min().unwrap();
            Indent {
                n: min,
                kind: IndentKind::Space,
            }
        } else {
            let min = tabs.iter().map(|i| i.n).min().unwrap();
            Indent {
                n: min,
                kind: IndentKind::Tab,
            }
        }
    }
}

impl Default for Indent {
    fn default() -> Self {
        Indent {
            n: 4,
            kind: IndentKind::Space,
        }
    }
}

fn indent_from_bytes(bytes: &mut Bytes) -> Indent {
    use IndentKind::*;
    let kind = bytes
        .next()
        .map(|b| {
            if b == Tab as u8 {
                Some(Tab)
            } else if b == Space as u8 {
                Some(Space)
            } else {
                None
            }
        })
        .flatten();

    match kind {
        Some(Tab) => {
            let mut n = 1;
            while let Some(b) = bytes.next() {
                if b == Tab as u8 {
                    n += 1;
                } else {
                    break;
                }
            }

            Indent { n, kind: Tab }
        }
        Some(Space) => {
            let mut n = 1;
            while let Some(b) = bytes.next() {
                if b == Space as u8 {
                    n += 1;
                } else {
                    break;
                }
            }

            Indent { n, kind: Space }
        }
        None => Indent { n: 0, kind: Space },
    }
}

// TODO replace with at_indent
/// Calculate indentation level at a line where pos resides
pub(crate) fn indent_at(slice: &PieceTreeSlice, pos: usize) -> Indent {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }

    indent_from_bytes(&mut bytes)
}

// TODO replace with at_indent
/// Returns how much we can dedent the line from pos
pub(crate) fn can_dedent(slice: &PieceTreeSlice, pos: usize) -> Option<usize> {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }
    let start = bytes.pos();
    let at_start = bytes.pos() == pos;
    if !at_start {
        return None;
    }

    indent_from_bytes(&mut bytes);
    if bytes.pos() != pos + 1 {
        return None;
    }

    Some(pos - start)
}

/// If pos is at indentation = just indentation to the left of cursor
/// returns the indentation left of cursor
pub(crate) fn at_indent(slice: &PieceTreeSlice, pos: usize) -> Option<Indent> {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }

    let start = bytes.pos();
    let mut indent = indent_from_bytes(&mut bytes);
    let end = bytes.pos();

    if !(start..end).contains(&pos) {
        return None;
    }

    indent.n = pos - start;
    Some(indent)
}
