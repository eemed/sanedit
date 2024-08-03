use documented::DocumentedFields;
use serde::{Deserialize, Serialize};

use std::cmp::min;

use sanedit_buffer::{utf8::prev_eol, Bytes, PieceTreeSlice};

use crate::editor::buffers::Options;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum IndentKind {
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

// #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, DocumentedFields, Serialize, Deserialize)]
// pub(crate) struct Indent {
//     /// Indent options, overridden if detect_indent is set
//     /// Available options:
//     /// Space: use spaces
//     /// Tab: use tabs
//     kind: IndentKind,

//     /// How many indent characters a single indent should be
//     count: usize,
// }

// impl Indent {
//     /// How much this indentation would need to be added so that it would be
//     /// a multiple of n. This tries to always indent even if self is already
//     /// multiple of n
//     pub fn indent_to_multiple_of(&self, n: usize) -> usize {
//         let mut res = self.count % n;
//         if res == 0 {
//             res = n;
//         }

//         res
//     }

//     /// How much this indentation would need to be removed so that it would be
//     /// a multiple of n. This tries to always dedent even if self is already
//     /// multiple of n
//     pub fn dedent_to_multiple_of(&self, n: usize) -> usize {
//         let mut res = self.count % n;
//         if res == 0 {
//             res = min(self.count, n);
//         }
//         res
//     }

//     pub fn to_string(&self) -> String {
//         self.kind.as_str().repeat(self.count)
//     }

//     pub fn determine(slice: &PieceTreeSlice) -> Indent {
//         let mut indents = vec![];
//         let mut lines = slice.lines();
//         while let Some(line) = lines.next() {
//             let mut bytes = line.bytes();
//             let indent = indent_from_bytes(&mut bytes);
//             if indent.count != 0 {
//                 indents.push(indent);
//             }
//         }

//         // Default
//         if indents.is_empty() {
//             return Indent::default();
//         }

//         let (spaces, tabs): (Vec<Indent>, Vec<Indent>) = indents
//             .into_iter()
//             .partition(|indent| indent.kind == IndentKind::Space);

//         if spaces.len() >= tabs.len() {
//             let min = spaces.iter().map(|i| i.count).min().unwrap();
//             Indent {
//                 count: min,
//                 kind: IndentKind::Space,
//             }
//         } else {
//             let min = tabs.iter().map(|i| i.count).min().unwrap();
//             Indent {
//                 count: min,
//                 kind: IndentKind::Tab,
//             }
//         }
//     }
// }

// impl Default for Indent {
//     fn default() -> Self {
//         Indent {
//             count: 4,
//             kind: IndentKind::Space,
//         }
//     }
// }

pub fn determine_indent(slice: &PieceTreeSlice) -> (IndentKind, usize) {
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
        let opts = Options::default();
        return (opts.indent_kind, opts.indent_amount);
    }

    let (spaces, tabs): (Vec<(IndentKind, usize)>, Vec<(IndentKind, usize)>) = indents
        .into_iter()
        .partition(|(kind, _)| *kind == IndentKind::Space);

    if spaces.len() >= tabs.len() {
        let min = spaces.iter().map(|(_, i)| i).min().unwrap();
        (IndentKind::Space, *min)
    } else {
        let min = tabs.iter().map(|(_, i)| i).min().unwrap();
        (IndentKind::Tab, *min)
    }
}

fn indent_from_bytes(bytes: &mut Bytes) -> Option<(IndentKind, usize)> {
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
pub(crate) fn indent_at_line(slice: &PieceTreeSlice, pos: usize) -> Option<(IndentKind, usize)> {
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
pub(crate) fn is_indent_at_pos(slice: &PieceTreeSlice, pos: usize) -> bool {
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
