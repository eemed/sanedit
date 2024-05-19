use sanedit_buffer::{utf8::prev_eol, Bytes, PieceTree, PieceTreeSlice};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum IndentKind {
    Space,
    Tab,
}

#[derive(Debug)]
pub(crate) struct Indent {
    pub(crate) n: usize,
    pub(crate) kind: IndentKind,
}

impl Indent {
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
    let kind = bytes
        .next()
        .map(|b| match b {
            b'\t' => Some(IndentKind::Tab),
            b' ' => Some(IndentKind::Space),
            _ => None,
        })
        .flatten();

    match kind {
        Some(IndentKind::Tab) => {
            let mut n = 1;
            while let Some(b) = bytes.next() {
                if b == b'\t' {
                    n += 1;
                } else {
                    break;
                }
            }

            Indent {
                n,
                kind: IndentKind::Tab,
            }
        }
        Some(IndentKind::Space) => {
            let mut n = 1;
            while let Some(b) = bytes.next() {
                if b == b' ' {
                    n += 1;
                } else {
                    break;
                }
            }

            Indent {
                n,
                kind: IndentKind::Space,
            }
        }
        None => Indent {
            n: 0,
            kind: IndentKind::Space,
        },
    }
}

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
