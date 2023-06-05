use std::ops::Range;

use sanedit_buffer::{Bytes, PieceTreeSlice};

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
