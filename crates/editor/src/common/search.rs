use aho_corasick::{AhoCorasick, StreamFindIter};
use std::ops::Range;

use sanedit_buffer::piece_tree::{PieceTreeSlice, SliceReader};

pub(crate) struct Searcher {
    ac: AhoCorasick,
}

impl Searcher {
    pub fn new(needle: &[u8]) -> Searcher {
        Searcher {
            ac: AhoCorasick::new([needle]).unwrap(),
        }
    }

    pub fn find_iter<'s, 'a: 's>(&'a mut self, slice: &'s PieceTreeSlice) -> SearchIter<'s> {
        let mut iter = self
            .ac
            .try_stream_find_iter(slice.reader())
            .expect("Failed to create stream iter");
        SearchIter { inner: iter }
    }
}

pub(crate) struct SearchIter<'a> {
    inner: StreamFindIter<'a, SliceReader<'a>>,
}

impl<'a> Iterator for SearchIter<'a> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()?.ok()
    }
}

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

// pub(crate) fn search_iter<'n, 's>(
//     needle: &'n [u8],
//     slice: &'s PieceTreeSlice,
// ) -> Option<SearchIter<'n, 's>> {
//     if needle.is_empty() {
//         return None;
//     }

//     let ac = AhoCorasick::new([needle]).unwrap();
//     let iter = ac
//         .try_stream_find_iter(slice.reader())
//         .expect("Failed to create stream iter");
//     Some(SearchIter { inner: iter })
// }
