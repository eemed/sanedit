use std::{cmp::max, ops::Range};

use crate::{Bytes, PieceTreeSlice};

#[derive(Debug, Clone)]
pub struct Searcher {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    good_suffix: Box<[usize]>,
    case_sensitive: bool,
}

impl Searcher {
    pub fn new(pattern: &[u8]) -> Searcher {
        Searcher {
            bad_char: Self::build_bad_char_table(pattern),
            good_suffix: Self::build_good_suffix_table(pattern),
            pattern: pattern.into(),
            case_sensitive: true,
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
        searcher.case_sensitive = false;

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
            let slen = Self::common_suffix_len(pattern, &pattern[..i + 1]);
            table[last - slen] = last + slen - i;
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
        SearchIter::new(
            &self.pattern,
            &self.bad_char,
            &self.good_suffix,
            slice,
            self.case_sensitive,
        )
    }

    pub fn pattern_len(&self) -> usize {
        self.pattern.len()
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

#[derive(Debug)]
pub struct SearchIter<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    good_suffix: &'a [usize],
    slice_len: u64,
    bytes: Bytes<'b>,
    i: u64,
    case_sensitive: bool,
}

impl<'a, 'b> SearchIter<'a, 'b> {
    pub fn new(
        pattern: &'a [u8],
        bad_char: &'a [usize],
        good_suffix: &'a [usize],
        slice: &'b PieceTreeSlice,
        case_sensitive: bool,
    ) -> SearchIter<'a, 'b> {
        SearchIter {
            pattern,
            bad_char,
            good_suffix,
            case_sensitive,
            slice_len: slice.len(),
            bytes: slice.bytes(),
            i: (pattern.len() - 1) as u64,
        }
    }

    fn find_next(&mut self) -> Option<Range<u64>> {
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

            while bytes.at(*i) == pattern[j] {
                if j == 0 {
                    *i += m as u64;
                    return Some(*i - m as u64..*i);
                }

                j -= 1;
                *i -= 1;
            }

            *i += max(bad_char[bytes.at(*i) as usize], good_suffix[j]) as u64;
        }

        None
    }

    fn find_next_insensitive(&mut self) -> Option<Range<u64>> {
        fn lower(mut byte: u8) -> u8 {
            byte.make_ascii_lowercase();
            byte
        }

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

            while lower(bytes.at(*i)) == pattern[j] {
                if j == 0 {
                    *i += m as u64;
                    return Some(*i - m as u64..*i);
                }

                j -= 1;
                *i -= 1;
            }

            *i += max(bad_char[lower(bytes.at(*i)) as usize], good_suffix[j]) as u64;
        }

        None
    }
}

impl<'a, 'b> Iterator for SearchIter<'a, 'b> {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.case_sensitive {
            self.find_next()
        } else {
            self.find_next_insensitive()
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearcherRev {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    good_suffix: Box<[usize]>,
    case_sensitive: bool,
}

impl SearcherRev {
    pub fn new(pattern: &[u8]) -> SearcherRev {
        SearcherRev {
            bad_char: Self::build_bad_char_table(pattern),
            good_suffix: Self::build_good_suffix_table(pattern),
            pattern: pattern.into(),
            case_sensitive: true,
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
        searcher.case_sensitive = false;

        Some(searcher)
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
        let mut table = [pattern.len(); 256];

        for i in (0..pattern.len()).rev() {
            table[pattern[i] as usize] = i;
        }

        table
    }

    fn build_good_suffix_table(pattern: &[u8]) -> Box<[usize]> {
        let mut table: Box<[usize]> = vec![0; pattern.len()].into();
        let mut last_suffix = 0;

        for i in 0..pattern.len() {
            if Self::is_suffix(pattern, &pattern[..i]) {
                last_suffix = i;
            }
            table[i] = pattern.len() - last_suffix + i;
        }

        for i in 1..pattern.len() {
            let slen = Self::common_prefix_len(pattern, &pattern[i..]);
            table[slen] = pattern.len() - i;
        }

        table
    }

    fn common_prefix_len(pattern: &[u8], other: &[u8]) -> usize {
        let mut i = 0;
        let plen = pattern.len();
        let olen = other.len();

        while i < plen && i < olen {
            if pattern[i] != other[i] {
                break;
            }

            i += 1;
        }

        i
    }

    fn is_suffix(pattern: &[u8], suffix: &[u8]) -> bool {
        if pattern.len() < suffix.len() {
            return false;
        }

        for i in 0..suffix.len() {
            if pattern[pattern.len() - 1 - i] != suffix[suffix.len() - 1 - i] {
                return false;
            }
        }

        true
    }

    pub fn find_iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> SearchIterRev<'a, 'b> {
        SearchIterRev::new(
            &self.pattern,
            &self.bad_char,
            &self.good_suffix,
            slice,
            self.case_sensitive,
        )
    }

    pub fn pattern_len(&self) -> usize {
        self.pattern.len()
    }
}

#[derive(Debug)]
pub struct SearchIterRev<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    good_suffix: &'a [usize],
    bytes: Bytes<'b>,
    i: u64,
    case_sensitive: bool,
}

impl<'a, 'b> SearchIterRev<'a, 'b> {
    pub fn new(
        pattern: &'a [u8],
        bad_char: &'a [usize],
        good_suffix: &'a [usize],
        slice: &'b PieceTreeSlice,
        case_sensitive: bool,
    ) -> SearchIterRev<'a, 'b> {
        SearchIterRev {
            pattern,
            bad_char,
            good_suffix,
            bytes: slice.bytes_at(slice.len()),
            i: slice.len().saturating_sub(pattern.len() as u64),
            case_sensitive,
        }
    }

    fn find_next(&mut self) -> Option<Range<u64>> {
        let SearchIterRev {
            pattern,
            bad_char,
            good_suffix,
            bytes,
            i,
            ..
        } = self;

        let m = pattern.len();
        let mut check0 = false;

        while *i != 0 || check0 {
            let mut j = 0;

            while bytes.at(*i) == pattern[j] {
                if j == m - 1 {
                    let end = *i + 1;
                    let start = end - m as u64;
                    *i = i.saturating_sub(m as u64);
                    return Some(start..end);
                }

                j += 1;
                *i += 1;
            }

            check0 = *i != 0;
            let shift = max(bad_char[bytes.at(*i) as usize], good_suffix[j]) as u64;
            *i = i.saturating_sub(shift);
        }

        None
    }

    fn find_next_insensitive(&mut self) -> Option<Range<u64>> {
        fn lower(mut byte: u8) -> u8 {
            byte.make_ascii_lowercase();
            byte
        }

        let SearchIterRev {
            pattern,
            bad_char,
            good_suffix,
            bytes,
            i,
            ..
        } = self;

        let m = pattern.len();
        let mut check0 = false;

        while *i != 0 || check0 {
            let mut j = 0;

            while lower(bytes.at(*i)) == pattern[j] {
                if j == m - 1 {
                    let end = *i + 1;
                    let start = end - m as u64;
                    *i = i.saturating_sub(m as u64);
                    return Some(start..end);
                }

                j += 1;
                *i += 1;
            }

            check0 = *i != 0;
            let shift = max(bad_char[lower(bytes.at(*i)) as usize], good_suffix[j]) as u64;
            *i = i.saturating_sub(shift);
        }

        None
    }
}

impl<'a, 'b> Iterator for SearchIterRev<'a, 'b> {
    type Item = Range<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.case_sensitive {
            self.find_next()
        } else {
            self.find_next_insensitive()
        }
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
    fn search_bwd() {
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
}
