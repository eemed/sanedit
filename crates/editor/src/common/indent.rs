use sanedit_buffer::{utf8::prev_eol, PieceTreeSlice};

#[derive(Debug)]
pub(crate) enum IndentKind {
    Space,
    Tab,
}

#[derive(Debug)]
pub(crate) struct Indent {
    level: usize,
    kind: IndentKind,
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
            let mut level = 1;
            while let Some(b) = bytes.next() {
                if b == b'\t' {
                    level += 1;
                } else {
                    break;
                }
            }

            Indent {
                level,
                kind: IndentKind::Tab,
            }
        }
        Some(IndentKind::Space) => {
            let mut level = 1;
            while let Some(b) = bytes.next() {
                if b == b' ' {
                    level += 1;
                } else {
                    break;
                }
            }

            Indent {
                level,
                kind: IndentKind::Space,
            }
        }
        None => Indent {
            level: 0,
            kind: IndentKind::Space,
        },
    }
}
