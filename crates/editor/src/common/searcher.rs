use aho_corasick::AhoCorasick;
use std::ops::Range;

use sanedit_buffer::piece_tree::PieceTreeSlice;

pub(crate) struct Searcher<'a> {
    haystack: PieceTreeSlice<'a>,
}

impl<'a> Searcher<'a> {
    pub fn new(slice: PieceTreeSlice<'a>) -> Searcher {
        Searcher { haystack: slice }
    }

    /// find the first match of needle forwards
    pub fn find<B: AsRef<[u8]>>(&mut self, needle: B) -> Option<Range<usize>> {
        let bytes = needle.as_ref();
        if bytes.is_empty() {
            return None;
        }

        let ac = AhoCorasick::new(&[bytes]).unwrap();
        let mut iter = ac
            .try_stream_find_iter(self.haystack.reader())
            .expect("Failed to create stream iter");
        let mat = iter.next()?.expect("failed to get match");
        let range = mat.span().range();
        Some(range)
    }

    /// find the first match of needle backwards
    pub fn find_back<B: AsRef<[u8]>>(&mut self, needle: B) -> Option<Range<usize>> {
        None
    }

    pub fn find_iter<B: AsRef<[u8]>>(&mut self, needle: B) -> () {
        todo!()
    }

    pub fn find_iter_back(&mut self) -> Vec<Range<usize>> {
        vec![]
    }
}
