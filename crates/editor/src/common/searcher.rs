use aho_corasick::{AhoCorasick, Input, PatternID};
use std::ops::Range;

use sanedit_buffer::piece_tree::PieceTreeSlice;

pub(crate) struct Searcher<'a> {
    haystack: PieceTreeSlice<'a>,
}

impl<'a> Searcher<'a> {
    pub fn new<B: AsRef<[u8]> + 'a>(slice: PieceTreeSlice<'a>) -> Searcher {
        Searcher { haystack: slice }
    }

    /// find the first match of needle forwards
    pub fn find<B: AsRef<[u8]>>(&mut self, needle: B) -> Option<Range<usize>> {
        let ac = AhoCorasick::new(&[needle]).unwrap();
        let mat = ac.find(&self.haystack)?;
        let range = mat.span().range();
        Some(range)
    }

    /// find the first match of needle backwards
    pub fn find_back<B: AsRef<[u8]>>(&mut self, needle: B) -> Option<Range<usize>> {
        None
    }

    pub fn find_iter(&mut self) -> Vec<Range<usize>> {
        vec![]
    }

    pub fn find_iter_back(&mut self) -> Vec<Range<usize>> {
        vec![]
    }
}

// impl<'h> From<&PieceTreeSlice<'h>> for Input<'h> {
// }
