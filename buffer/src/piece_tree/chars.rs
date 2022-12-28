use std::ops::Range;

use super::{Bytes, PieceTree, chunks::Chunks};

const REPLACEMENT: char = '\u{FFFD}';

#[derive(Debug, Clone)]
pub struct Chars<'a> {
    pt: &'a PieceTree,
    chunks: Chunks<'a>,
}

impl<'a> Chars<'a> {
    pub fn new(pt: &'a PieceTree, at: usize) -> Chars<'a> {
        let chunks = Chunks::new(pt, at);
        Chars {
            pt,
            chunks,
        }
        // use similar to graphemes or is this better?
        // bstr::decode_utf8();
    }

    pub fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Chars<'a> {
        let chunks = Chunks::new_from_slice(pt, at, range);
        Chars {
            pt,
            chunks,
        }
    }

    pub fn next(&mut self) -> (usize, char) {
        todo!()
    }

    pub fn prev(&mut self) -> (usize, char) {
        todo!()
    }
}
