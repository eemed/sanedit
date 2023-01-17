pub(crate) mod chars;
pub(crate) mod graphemes;
// pub(crate) mod lines;

use super::{slice::PieceTreeSlice, PieceTree};

impl From<&PieceTree> for String {
    fn from(pt: &PieceTree) -> Self {
        let mut result = String::new();
        let mut chars = pt.chars();
        while let Some((_, ch)) = chars.next() {
            result.push(ch);
        }
        result
    }
}

impl<'a> From<&PieceTreeSlice<'a>> for String {
    fn from(slice: &PieceTreeSlice) -> Self {
        let mut result = String::new();
        let mut chars = slice.chars();
        while let Some((_, ch)) = chars.next() {
            result.push(ch);
        }
        result
    }
}
