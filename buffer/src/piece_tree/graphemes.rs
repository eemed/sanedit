use crate::cursor_iterator::CursorIterator;

use bstr::ByteSlice;
use super::PieceTree;

pub fn is_grapheme_boundary(pt: &PieceTree, pos: usize) -> bool {
    // let chunks = pt.chunks_at(pos);
    // let chunk = chunks.get();

    // if let Some(chk) = chunk {
    //     let bytes = chk.as_ref();
    // } else {
    //     false
    // }
    todo!()
}

pub fn next_grapheme_boundary(pt: &PieceTree, pos: usize) -> Option<usize> {
    todo!()
}

pub fn prev_grapheme_boundary(pt: &PieceTree, pos: usize) -> Option<usize> {
    todo!()
}

pub fn next_grapheme(pt: &PieceTree, pos: usize) -> Option<(usize, usize, String)> {
    todo!()
}

pub fn prev_grapheme(pt: &PieceTree, pos: usize) -> Option<(usize, usize, String)> {
    todo!()
}
