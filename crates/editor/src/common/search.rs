use aho_corasick::AhoCorasick;
use std::ops::Range;

use sanedit_buffer::piece_tree::PieceTreeSlice;

pub(crate) fn search(needle: &[u8], slice: &PieceTreeSlice) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }

    let ac = AhoCorasick::new([needle]).unwrap();
    let mut iter = ac
        .try_stream_find_iter(slice.reader())
        .expect("Failed to create stream iter");
    let mat = iter.next()?.expect("failed to get match");
    let range = mat.span().range();
    Some(range)
}

pub(crate) fn search_all(needle: &[u8], slice: &PieceTreeSlice) -> Option<Range<usize>> {
    None
}
