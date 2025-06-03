use std::{
    cmp::max,
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::{Bytes, PieceTreeSlice};

#[derive(Debug, Clone)]
pub struct Searcher {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    byte_at: fn(&mut Bytes, u64) -> u8,
}

impl Searcher {
    pub fn new(pattern: &[u8]) -> Searcher {
        Searcher {
            bad_char: Self::build_bad_char_table(pattern),
            pattern: pattern.into(),
            byte_at,
        }
    }

    /// Create a new ascii case insensitive searcher
    /// Pattern must be all ascii characters otherwise none will be returned
    pub fn new_ascii_case_insensitive(patt: &str) -> Option<Searcher> {
        if !patt.is_ascii() {
            return None;
        }

        let patt = patt.to_ascii_lowercase();
        let mut searcher = Self::new(patt.as_bytes());
        searcher.byte_at = byte_at_lower;

        Some(searcher)
    }

    fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
        let mut table = [pattern.len(); 256];
        let last = pattern.len() - 1;

        for i in 0..last {
            table[pattern[i] as usize] = last - i;
        }

        table
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> SearchIter<'a, 'b> {
        self.find_iter_stoppable(slice, Arc::new(AtomicBool::new(false)))
    }

    pub fn find_iter_stoppable<'a, 'b: 'a>(
        &'a self,
        slice: &'b PieceTreeSlice,
        stop: Arc<AtomicBool>,
    ) -> SearchIter<'a, 'b> {
        SearchIter {
            pattern: &self.pattern,
            bad_char: &self.bad_char,
            stop,
            slice_len: slice.len(),
            bytes: slice.bytes(),
            i: (self.pattern.len() - 1) as u64,
            byte_at,
        }
    }

    pub fn pattern_len(&self) -> usize {
        self.pattern.len()
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.byte_at == byte_at
    }
}

fn byte_at_lower(bytes: &mut Bytes, i: u64) -> u8 {
    let mut byte = bytes.at(i);
    byte.make_ascii_lowercase();
    byte
}

fn byte_at(bytes: &mut Bytes, i: u64) -> u8 {
    bytes.at(i)
}

#[derive(Debug, Clone)]
pub struct SearchIter<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    slice_len: u64,
    bytes: Bytes<'b>,
    i: u64,
    byte_at: fn(&mut Bytes, u64) -> u8,
    stop: Arc<AtomicBool>,
}

impl<'a, 'b> Iterator for SearchIter<'a, 'b> {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        let SearchIter {
            pattern,
            bad_char,
            slice_len,
            bytes,
            i,
            stop,
            byte_at,
            ..
        } = self;

        let m = pattern.len();
        let n = *slice_len;

        while *i < n {
            if stop.load(Ordering::Acquire) {
                return None;
            }
            let mut j = m - 1;

            while byte_at(bytes, *i) == pattern[j] {
                if j == 0 {
                    *i += m as u64;
                    return Some(*i - m as u64..*i);
                }

                j -= 1;
                *i -= 1;
            }

            *i += max(m - j, bad_char[bytes.at(*i) as usize]) as u64;
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct SearcherRev {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    byte_at: fn(&mut Bytes, u64) -> u8,
}

impl SearcherRev {
    pub fn new(pattern: &[u8]) -> SearcherRev {
        SearcherRev {
            bad_char: Self::build_bad_char_table(pattern),
            pattern: pattern.into(),
            byte_at,
        }
    }

    /// Create a new ascii case insensitive searcher
    /// Pattern must be all ascii characters otherwise none will be returned
    pub fn new_ascii_case_insensitive(patt: &str) -> Option<SearcherRev> {
        if !patt.is_ascii() {
            return None;
        }

        let patt = patt.to_ascii_lowercase();
        let mut searcher = Self::new(patt.as_bytes());
        searcher.byte_at = byte_at_lower;

        Some(searcher)
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.byte_at == byte_at
    }

    fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
        let mut table = [pattern.len(); 256];

        for i in (0..pattern.len()).rev() {
            table[pattern[i] as usize] = i;
        }

        table
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> SearchIterRev<'a, 'b> {
        self.find_iter_stoppable(slice, Arc::new(AtomicBool::new(false)))
    }

    pub fn find_iter_stoppable<'a, 'b: 'a>(
        &'a self,
        slice: &'b PieceTreeSlice,
        stop: Arc<AtomicBool>,
    ) -> SearchIterRev<'a, 'b> {
        SearchIterRev {
            pattern: &self.pattern,
            bad_char: &self.bad_char,
            bytes: slice.bytes_at(slice.len()),
            byte_at: self.byte_at,
            stop,
            i: slice.len().saturating_sub(self.pattern.len() as u64),
        }
    }

    pub fn pattern_len(&self) -> usize {
        self.pattern.len()
    }
}

#[derive(Debug, Clone)]
pub struct SearchIterRev<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    bytes: Bytes<'b>,
    i: u64,
    stop: Arc<AtomicBool>,
    byte_at: fn(&mut Bytes, u64) -> u8,
}

impl<'a, 'b> Iterator for SearchIterRev<'a, 'b> {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        let SearchIterRev {
            pattern,
            bad_char,
            bytes,
            i,
            stop,
            byte_at,
            ..
        } = self;

        let m = pattern.len();
        let mut continue_search = *i != 0;

        while continue_search {
            if stop.load(Ordering::Acquire) {
                return None;
            }
            // Continue until we are checking 0
            continue_search = *i != 0;
            let mut j = 0;

            while byte_at(bytes, *i) == pattern[j] {
                if j == m - 1 {
                    let end = *i + 1;
                    let start = end - m as u64;
                    *i = i.saturating_sub(m as u64);
                    return Some(start..end);
                }

                j += 1;
                *i += 1;
            }

            let shift = max(j + 1, bad_char[bytes.at(*i) as usize]) as u64;
            *i = i.saturating_sub(shift);
        }

        None
    }
}

#[cfg(test)]
mod test {
    use crate::piece_tree::PieceTree;

    use super::*;

    #[test]
    fn search_fwd() {
        let pt = PieceTree::from("[dependencies][dev-dependencies]");

        let needle = b"dependencies";
        let searcher = Searcher::new(needle);
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);
        assert_eq!(Some(1..13), iter.next());
        assert_eq!(Some(19..31), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn search_fwd2() {
        let pt = PieceTree::from("dependenciesdependencies");

        let needle = b"dependencies";
        let searcher = Searcher::new(needle);
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);
        assert_eq!(Some(0..12), iter.next());
        assert_eq!(Some(12..24), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn search_bwd1() {
        let pt = PieceTree::from("[dependencies][dev-dependencies]");

        let needle = b"dependencies";
        let searcher = SearcherRev::new(needle);
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);

        assert_eq!(Some(19..31), iter.next());
        assert_eq!(Some(1..13), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn search_bwd2() {
        let pt = PieceTree::from("dependenciesdependencies");

        let needle = b"dependencies";
        let searcher = SearcherRev::new(needle);
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);

        assert_eq!(Some(12..24), iter.next());
        assert_eq!(Some(0..12), iter.next());
        assert_eq!(None, iter.next());
    }

    // #[test]
    // fn search_bwd3() {
    //     let err_file = include_str!("../../editor/src/actions/filetree.rs");
    //     let pt = PieceTree::from(err_file);
    //     let slice = pt.slice(..);
    //     let needle = b"remove_dir";

    //     let searcher = Searcher::new(needle);
    //     let mut iter = searcher.find_iter(&slice);
    //     while let Some(n) = iter.next() {
    //         println!("{n:?}");
    //     }

    //     let searcher = SearcherRev::new(needle);
    //     let mut iter = searcher.find_iter(&slice);

    //     while let Some(n) = iter.next() {
    //         println!("{n:?}");
    //     }
    // }
}
