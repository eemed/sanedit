use serde::{Deserialize, Serialize};

use std::cmp::min;

use sanedit_buffer::{utf8::prev_eol, Bytes, PieceTreeSlice};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct Indent {
    #[serde(rename = "indent_count")]
    pub(crate) n: usize,

    #[serde(rename = "indent")]
    pub(crate) kind: IndentKind,
}

impl Indent {
    pub fn get(&self, level: usize) -> String {
        self.kind.as_str().repeat(level * self.n)
    }

    /// How much this indentation would need to be added so that it would be
    /// a multiple of n. This tries to always indent even if self is already
    /// multiple of n
    pub fn indent_to_multiple_of(&self, n: usize) -> usize {
        let mut res = self.n % n;
        if res == 0 {
            res = n;
        }

        res
    }

    /// How much this indentation would need to be removed so that it would be
    /// a multiple of n. This tries to always dedent even if self is already
    /// multiple of n
    pub fn dedent_to_multiple_of(&self, n: usize) -> usize {
        let mut res = self.n % n;
        if res == 0 {
            res = min(self.n, n);
        }
        res
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

/// Calculate indentation level at a line where pos resides
pub(crate) fn indent_at_line(slice: &PieceTreeSlice, pos: usize) -> Indent {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }

    indent_from_bytes(&mut bytes)
}

/// If pos is at indentation = just indentation to the left of cursor
/// returns the indentation left of cursor
pub(crate) fn indent_at_pos(slice: &PieceTreeSlice, pos: usize) -> Option<Indent> {
    let mut bytes = slice.bytes_at(pos);
    if let Some(eol) = prev_eol(&mut bytes) {
        let len = eol.eol.len();
        for _ in 0..len {
            bytes.next();
        }
    }

    let start = bytes.pos();
    let indent = indent_from_bytes(&mut bytes);
    let end = start + indent.n;

    if end == slice.len() {
        return Some(indent);
    }

    if !(start..=end).contains(&pos) {
        return None;
    }

    Some(indent)
}
