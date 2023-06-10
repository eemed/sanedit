use std::{cmp::max, ops::Range};

use crate::{Bytes, PieceTreeSlice};

#[derive(Debug)]
pub struct Searcher {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    good_suffix: Box<[usize]>,
}

impl Searcher {
    pub fn new(pattern: &[u8]) -> Searcher {
        Searcher {
            bad_char: Self::build_bad_char_table(pattern),
            good_suffix: Self::build_good_suffix_table(pattern),
            pattern: pattern.into(),
        }
    }

    fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
        let mut table = [pattern.len(); 256];
        let last = pattern.len() - 1;

        for i in 0..last {
            table[pattern[i] as usize] = last - i;
        }

        table
    }

    fn build_good_suffix_table(pattern: &[u8]) -> Box<[usize]> {
        let mut table: Box<[usize]> = vec![0; pattern.len()].into();
        let last = pattern.len() - 1;
        let mut last_prefix = last;

        for i in (0..=last).rev() {
            if Self::is_prefix(pattern, &pattern[i + 1..]) {
                last_prefix = i + 1;
            }
            table[i] = last_prefix + last - i;
        }

        for i in 0..last {
            let slen = Self::common_suffix_len(pattern, &pattern[1..i + 1]);
            if pattern[i - slen] != pattern[last - slen] {
                table[last - slen] = last + slen - i;
            }
        }

        table
    }

    fn common_suffix_len(pattern: &[u8], other: &[u8]) -> usize {
        let mut i = 0;
        let plen = pattern.len();
        let olen = other.len();

        while i < plen && i < olen {
            if pattern[plen - 1 - i] != other[olen - 1 - i] {
                break;
            }

            i += 1;
        }

        i
    }

    fn is_prefix(pattern: &[u8], prefix: &[u8]) -> bool {
        if pattern.len() < prefix.len() {
            return false;
        }

        for i in 0..prefix.len() {
            if pattern[i] != prefix[i] {
                return false;
            }
        }

        true
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> SearchIter<'a, 'b> {
        SearchIter::new(&self.pattern, &self.bad_char, &self.good_suffix, slice)
    }
}

#[derive(Debug)]
pub struct SearchIter<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    good_suffix: &'a [usize],
    slice_len: usize,
    bytes: Bytes<'b>,
    i: usize,
}

impl<'a, 'b> SearchIter<'a, 'b> {
    pub fn new(
        pattern: &'a [u8],
        bad_char: &'a [usize],
        good_suffix: &'a [usize],
        slice: &'b PieceTreeSlice,
    ) -> SearchIter<'a, 'b> {
        SearchIter {
            pattern,
            bad_char,
            good_suffix,
            slice_len: slice.len(),
            bytes: slice.bytes(),
            i: pattern.len() - 1,
        }
    }
}

impl<'a, 'b> Iterator for SearchIter<'a, 'b> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let SearchIter {
            pattern,
            bad_char,
            good_suffix,
            slice_len,
            bytes,
            i,
            ..
        } = self;

        let m = pattern.len();
        let n = *slice_len;

        while *i < n {
            let mut j = m - 1;
            let mut mat = None;

            while bytes.at(*i) == pattern[j] {
                if j == 0 {
                    mat = Some(*i..*i + m);
                    *i += m;
                    break;
                }

                j -= 1;
                *i -= 1;
            }

            if *i < n {
                *i += max(bad_char[bytes.at(*i) as usize], good_suffix[j]);
            }

            if mat.is_some() {
                return mat;
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct SearcherRev {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    good_suffix: Box<[usize]>,
}

impl SearcherRev {
    pub fn new(pattern: &[u8]) -> SearcherRev {
        SearcherRev {
            bad_char: Self::build_bad_char_table(pattern),
            good_suffix: Self::build_good_suffix_table(pattern),
            pattern: pattern.into(),
        }
    }

    fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
        let mut table = [pattern.len(); 256];

        for i in 0..pattern.len() {
            table[pattern[i] as usize] = i;
        }

        table
    }

    fn build_good_suffix_table(pattern: &[u8]) -> Box<[usize]> {
        let mut table: Box<[usize]> = vec![0; pattern.len()].into();
        let last = pattern.len() - 1;
        let mut last_prefix = last;

        for i in (0..=last).rev() {
            if Self::is_prefix(pattern, &pattern[i + 1..]) {
                last_prefix = i + 1;
            }
            table[i] = last_prefix + last - i;
        }

        for i in 0..last {
            let slen = Self::common_suffix_len(pattern, &pattern[1..i + 1]);
            if pattern[i - slen] != pattern[last - slen] {
                table[last - slen] = last + slen - i;
            }
        }

        table
    }

    fn common_suffix_len(pattern: &[u8], other: &[u8]) -> usize {
        let mut i = 0;
        let plen = pattern.len();
        let olen = other.len();

        while i < plen && i < olen {
            if pattern[plen - 1 - i] != other[olen - 1 - i] {
                break;
            }

            i += 1;
        }

        i
    }

    fn is_prefix(pattern: &[u8], prefix: &[u8]) -> bool {
        if pattern.len() < prefix.len() {
            return false;
        }

        for i in 0..prefix.len() {
            if pattern[i] != prefix[i] {
                return false;
            }
        }

        true
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> SearchIterRev<'a, 'b> {
        SearchIterRev::new(&self.pattern, &self.bad_char, &self.good_suffix, slice)
    }
}

#[derive(Debug)]
pub struct SearchIterRev<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    good_suffix: &'a [usize],
    slice_len: usize,
    bytes: Bytes<'b>,
    i: usize,
}

impl<'a, 'b> SearchIterRev<'a, 'b> {
    pub fn new(
        pattern: &'a [u8],
        bad_char: &'a [usize],
        good_suffix: &'a [usize],
        slice: &'b PieceTreeSlice,
    ) -> SearchIterRev<'a, 'b> {
        SearchIterRev {
            pattern,
            bad_char,
            good_suffix,
            slice_len: slice.len(),
            bytes: slice.bytes_at(slice.len()),
            i: slice.len().saturating_sub(pattern.len()),
        }
    }
}

impl<'a, 'b> Iterator for SearchIterRev<'a, 'b> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let SearchIterRev {
            pattern,
            bad_char,
            bytes,
            i,
            ..
        } = self;

        let m = pattern.len();

        if *i == 0 {
            return None;
        }

        loop {
            let mut j = 0;
            let mut mat = None;

            while bytes.at(*i) == pattern[j] {
                if j == m - 1 {
                    mat = Some(*i + 1 - m..*i + 1);
                    *i = i.saturating_sub(m);
                    break;
                }

                j += 1;
                *i += 1;
            }

            // TODO: use good suffix table too
            *i = i.saturating_sub(max(bad_char[bytes.at(*i) as usize], 1));

            if mat.is_some() {
                return mat;
            } else if *i == 0 {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::piece_tree::PieceTree;

    use super::*;

    #[test]
    fn boyer_moore() {
        let mut pt = PieceTree::new();
        pt.insert(
            0,
            "world. This is another world. In another universe. Other worldy creatures. worl orld world",
        );

        let searcher = Searcher::new(b"world");
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);

        while let Some(it) = iter.next() {
            println!("BM-F: {it:?}");
        }

        println!("==========================");

        let searcher = SearcherRev::new(b"world");
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);

        while let Some(it) = iter.next() {
            println!("BM-B: {it:?}");
        }
    }
}
