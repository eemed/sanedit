use std::{
    cmp::{max, min},
    ops::Range,
};

use sanedit_buffer::piece_tree::{Bytes, PieceTreeSlice};

// TODO boyer-moore algo is faster http://igm.univ-mlv.fr/~lecroq/string/node14.html#SECTION00140
// it needs a sliding window of patterns length. Is it still faster even though
// we are copying all the text we are comparing?

#[derive(Debug)]
pub(crate) struct SearcherBM {
    pattern: Vec<u8>,
    bad_char: [usize; 256],
    good_suffix: Box<[usize]>,
}

impl SearcherBM {
    pub fn new(pattern: &[u8]) -> SearcherBM {
        // https://go.dev/src/strings/search.go
        SearcherBM {
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

        let mut last_prefix = pattern.len();
        for i in (0..=last).rev() {
            if Self::has_prefix(pattern, &pattern[i + 1..]) {
                last_prefix = i + 1;
            }
            table[last - i] = last_prefix + last - i;
        }

        for i in 0..last {
            let slen = Self::suffix_len(pattern, i);
            table[slen] = last - i + slen;
        }

        table
    }

    fn suffix_len(pattern: &[u8], p: usize) -> usize {
        let mut len = 0;
        let mut i = p;
        let mut j = pattern.len() - 1;

        while pattern[i] == pattern[j] {
            len += 1;

            if i == 0 {
                break;
            } else {
                i -= 1;
            }

            j -= 1;
        }

        len
    }

    fn has_prefix(pattern: &[u8], prefix: &[u8]) -> bool {
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

    fn iter<'a, 'b: 'a>(&'a self, slice: &'b PieceTreeSlice) -> SearchBMIter<'a, 'b> {
        SearchBMIter::new(&self.pattern, &self.bad_char, &self.good_suffix, slice)
    }
}

#[derive(Debug)]
struct SearchBMIter<'a, 'b> {
    pattern: &'a [u8],
    bad_char: &'a [usize],
    good_suffix: &'a [usize],
    slice_len: usize,
    bytes: Bytes<'b>,
    i: i64,
}

impl<'a, 'b> SearchBMIter<'a, 'b> {
    pub fn new(
        pattern: &'a [u8],
        bad_char: &'a [usize],
        good_suffix: &'a [usize],
        slice: &'b PieceTreeSlice,
    ) -> SearchBMIter<'a, 'b> {
        SearchBMIter {
            pattern,
            bad_char,
            good_suffix,
            slice_len: slice.len(),
            bytes: slice.bytes(),
            i: (pattern.len() - 1) as i64,
        }
    }
}

impl<'a, 'b> Iterator for SearchBMIter<'a, 'b> {
    type Item = Range<i64>;

    fn next(&mut self) -> Option<Self::Item> {
        let SearchBMIter {
            pattern,
            bad_char,
            good_suffix,
            slice_len,
            bytes,
            i,
        } = self;

        let m = pattern.len();

        while *i < *slice_len as i64 {
            let mut j = (m - 1) as i64;

            while bytes.byte_at(*i as usize) == pattern[j as usize] {
                if j == 0 {
                    let mat = *i..*i + m as i64;
                    *i += if *i + (m as i64) < *slice_len as i64 {
                        max(
                            bad_char[bytes.byte_at(*i as usize + m) as usize],
                            1, // good_suffix[j as usize],
                        ) as i64
                    } else {
                        m as i64
                    };
                    return Some(mat);
                }

                j -= 1;
                *i -= 1;
            }

            *i += max(
                bad_char[bytes.byte_at(*i as usize) as usize],
                1, // good_suffix[j as usize],
            ) as i64;
        }

        None
    }
}

#[cfg(test)]
mod test {
    use sanedit_buffer::piece_tree::PieceTree;

    use super::*;

    #[test]
    fn boyer_moore() {
        let mut pt = PieceTree::new();
        pt.insert(
            0,
            "world. This is another world. In another universe. Other worldy creatures. world",
        );

        let searcher = Searcher::new(b"world");
        let slice = pt.slice(..);
        let mut iter = searcher.find_iter(&slice);

        while let Some(it) = iter.next() {
            println!("KMP-F: {it:?}");
        }

        let searcher = SearcherBM::new(b"world");
        let slice = pt.slice(..);
        let mut iter = searcher.iter(&slice);

        while let Some(it) = iter.next() {
            println!("BM-F: {it:?}");
        }

        // let searcher = Searcher::new(b"aaaa");
        // println!("LPS: {:?}", searcher.lps);

        // let rsearcher = SearcherRev::new(b"aaaa");
        // let rsearcher = SearcherRev::new(b"world");
        // let mut iter = rsearcher.find_iter(&slice);

        // while let Some(it) = iter.next() {
        //     println!("BW: {it:?}");
        // }
    }
}

// ###############################
// ###############################
// ###############################
// ###############################

#[derive(Debug)]
pub(crate) struct SearcherRev {
    lps: Vec<usize>,
    pat: Vec<u8>,
}

impl SearcherRev {
    pub fn new(pat: &[u8]) -> SearcherRev {
        let pat: Vec<u8> = pat.iter().cloned().rev().collect();
        SearcherRev {
            lps: Searcher::kmp_precompute(&pat),
            pat,
        }
    }

    pub fn find_iter<'b, 'a: 'b>(&'b self, slice: &'a PieceTreeSlice) -> SearchIterRev<'a, 'b> {
        SearchIterRev::new(slice, &self.pat, &self.lps)
    }
}

pub(crate) struct SearchIterRev<'a, 'b> {
    pat: &'b [u8],
    lps: &'b [usize], // longest proper prefix which is also a suffix
    bytes: Bytes<'a>,
    byte: u8,
    i: usize,
    j: usize,
}

impl<'a, 'b> SearchIterRev<'a, 'b> {
    pub fn new(
        slice: &'a PieceTreeSlice,
        pat: &'b [u8],
        lps: &'b [usize],
    ) -> SearchIterRev<'a, 'b> {
        let mut bytes = slice.bytes_at(slice.len());
        let byte = bytes.prev().unwrap_or(0);

        SearchIterRev {
            lps,
            pat,
            bytes,
            byte,
            i: slice.len(),
            j: 0,
        }
    }
}

impl<'a, 'b> Iterator for SearchIterRev<'a, 'b> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let m = self.lps.len();
        let SearchIterRev {
            pat,
            lps,
            bytes,
            byte,
            i,
            j,
        } = self;

        while *i >= m - *j {
            if pat[*j] == *byte {
                *j += 1;
                *i -= 1;
                if let Some(b) = bytes.prev() {
                    *byte = b;
                }
            }

            if *j == m {
                let mat = *i..*i + *j;
                *j = lps[*j - 1];
                return Some(mat);
            } else if pat[*j] != *byte {
                if *j != 0 {
                    *j = lps[*j - 1];
                } else {
                    *i -= 1;
                    *byte = bytes.prev()?;
                }
            }
        }

        None
    }
}

// --------------------
//
#[derive(Debug)]
pub(crate) struct Searcher {
    lps: Vec<usize>,
    pat: Vec<u8>,
}

impl Searcher {
    pub fn new(pat: &[u8]) -> Searcher {
        Searcher {
            lps: Self::kmp_precompute(pat),
            pat: pat.into(),
        }
    }

    fn kmp_precompute(pat: &[u8]) -> Vec<usize> {
        if pat.len() == 0 {
            return vec![];
        }

        let mut len = 0;
        let m = pat.len();
        let mut lps = vec![0; m];
        lps[0] = 0;

        let mut i = 1;
        while i < m {
            if pat[i] == pat[len] {
                len += 1;
                lps[i] = len;
                i += 1;
            } else {
                if len != 0 {
                    len = lps[len - 1];
                } else {
                    lps[i] = 0;
                    i += 1;
                }
            }
        }

        lps
    }

    pub fn find_iter<'b, 'a: 'b>(&'b self, slice: &'a PieceTreeSlice) -> SearchIter<'a, 'b> {
        SearchIter::new(slice, &self.pat, &self.lps)
    }
}

pub(crate) struct SearchIter<'a, 'b> {
    pat: &'b [u8],
    lps: &'b [usize], // longest proper prefix which is also a suffix
    bytes: Bytes<'a>,
    byte: u8,
    n: usize,
    i: usize,
    j: usize,
}

impl<'a, 'b> SearchIter<'a, 'b> {
    pub fn new(slice: &'a PieceTreeSlice, pat: &'b [u8], lps: &'b [usize]) -> SearchIter<'a, 'b> {
        let mut bytes = slice.bytes();
        let byte = bytes.next().unwrap_or(0);

        SearchIter {
            lps,
            pat,
            bytes,
            byte,
            n: slice.len(),
            i: 0,
            j: 0,
        }
    }
}

impl<'a, 'b> Iterator for SearchIter<'a, 'b> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let m = self.lps.len();
        let SearchIter {
            pat,
            lps,
            bytes,
            byte,
            n,
            i,
            j,
        } = self;

        //     i        n
        // |------------|
        //
        //    j    m
        // |-------|
        while *n - *i >= m - *j {
            if pat[*j] == *byte {
                *j += 1;
                *i += 1;
                if let Some(b) = bytes.next() {
                    *byte = b;
                }
            }

            if *j == m {
                let mat = *i - *j..*i;
                *j = lps[*j - 1];
                return Some(mat);
            } else if pat[*j] != *byte {
                if *j != 0 {
                    *j = lps[*j - 1];
                } else {
                    *i += 1;
                    *byte = bytes.next()?;
                }
            }
        }

        None
    }
}
