use bstr::ByteSlice;
use std::{
    cmp::max,
    io::{self, Read, Seek, SeekFrom},
};

use crate::source::Source;

const BUF_SIZE: usize = 128 * 1024;

#[derive(Debug, Clone)]
pub struct Finder {
    searcher: bstr::Finder<'static>,
    case_insensitive: bool,
}

impl Finder {
    pub fn new(pattern: &[u8]) -> Self {
        Finder {
            searcher: bstr::Finder::new(pattern).into_owned(),
            case_insensitive: false,
        }
    }

    pub fn new_case_insensitive(pattern: &[u8]) -> Self {
        Finder {
            searcher: bstr::Finder::new(pattern).into_owned(),
            case_insensitive: true,
        }
    }

    pub fn iter<S: Source>(&self, haystack: S) -> FinderIter<'_, S> {
        let leading_case_insentive = if self.is_case_sensitive() {
            None
        } else {
            let mut upper = self.needle()[0];
            upper.make_ascii_uppercase();

            let mut lower = self.needle()[0];
            lower.make_ascii_lowercase();
            Some([lower, upper])
        };

        FinderIter {
            haystack,
            finder: self,
            pos: 0,
            leading_case_insentive,
        }
    }

    pub fn is_case_sensitive(&self) -> bool {
        !self.case_insensitive
    }

    pub fn needle(&self) -> &[u8] {
        self.searcher.needle()
    }
}

#[derive(Debug)]
pub struct FinderIter<'a, S: Source> {
    haystack: S,
    pos: u64,
    finder: &'a Finder,
    leading_case_insentive: Option<[u8; 2]>,
}

impl<'a, S: Source> FinderIter<'a, S> {
    fn find_in_slice(&self, data: &[u8]) -> Option<usize> {
        if let Some(ref case_insensitive_set) = self.leading_case_insentive {
            let pattern = self.finder.needle();
            let mut n = 0;
            while n < data.len() {
                let slice = &data[n..];
                n += slice.find_byteset(case_insensitive_set)?;
                let end = n + pattern.len();
                if end > data.len() {
                    return None;
                }

                let candidate_data = &data[n..end];
                if self.finder.needle().eq_ignore_ascii_case(candidate_data) {
                    return Some(n);
                } else {
                    n += 1;
                }
            }

            None
        } else {
            self.finder.searcher.find(data)
        }
    }

    pub fn find_next(&mut self) -> Option<u64> {
        let pattern = self.finder.needle();
        let (buf_pos, buf) = self.haystack.buffer();
        let buf_end = buf_pos + (buf.len() as u64);

        loop {
            let remaining = (buf_end - self.pos) as usize;
            if remaining < pattern.len() && self.haystack.refill_buffer(self.pos).is_err() {
                return None;
            }

            let (buf_pos, buf) = self.haystack.buffer();
            // let buf_end = buf_pos + (buf.len() as u64);
            let relative_start = (self.pos - buf_pos) as usize;
            let relative_end = relative_start + (buf_pos + buf.len() as u64 - self.pos) as usize;

            let search_slice = &buf[relative_start..relative_end];

            if search_slice.len() < pattern.len() {
                return None;
            }

            if let Some(relative_pos) = self.find_in_slice(search_slice) {
                let absolute_pos = self.pos + relative_pos as u64;

                let advance_by = (relative_pos + pattern.len()) as u64;
                self.pos += advance_by;

                return Some(absolute_pos);
            } else {
                let advance_by = search_slice.len().saturating_sub(pattern.len() - 1) as u64;
                self.pos += advance_by;
            }
        }
    }
}

impl<'a, S: Source> Iterator for FinderIter<'a, S> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.find_next()
    }
}

#[derive(Debug, Clone)]
pub struct FinderRev {
    searcher: bstr::FinderReverse<'static>,
    case_insensitive: bool,
}

impl FinderRev {
    pub fn new(pattern: &[u8]) -> Self {
        FinderRev {
            searcher: bstr::FinderReverse::new(pattern).into_owned(),
            case_insensitive: false,
        }
    }

    pub fn new_case_insensitive(pattern: &[u8]) -> Self {
        FinderRev {
            searcher: bstr::FinderReverse::new(pattern).into_owned(),
            case_insensitive: true,
        }
    }

    pub fn iter<S: Source>(&self, haystack: S) -> io::Result<FinderIterRev<'_, S>> {
        FinderIterRev::new(haystack, self)
    }

    pub fn is_case_sensitive(&self) -> bool {
        !self.case_insensitive
    }

    pub fn needle(&self) -> &[u8] {
        self.searcher.needle()
    }
}

#[derive(Debug)]
pub struct FinderIterRev<'a, S: Source> {
    haystack: S,
    finder: &'a FinderRev,
    pos: u64,
    leading_case_insentive: Option<[u8; 2]>,
}

impl<'a, S: Source> FinderIterRev<'a, S> {
    fn new(haystack: S, finder: &'a FinderRev) -> io::Result<Self> {
        let file_size = haystack.len();

        let leading_case_insentive = if finder.is_case_sensitive() {
            None
        } else {
            let mut upper = finder.needle()[0];
            upper.make_ascii_uppercase();

            let mut lower = finder.needle()[0];
            lower.make_ascii_lowercase();
            Some([lower, upper])
        };

        Ok(Self {
            haystack,
            finder,
            pos: file_size,
            leading_case_insentive,
        })
    }

    fn refill_buffer_rev(&mut self) -> io::Result<bool> {
        // let remaining = self.buf_len as usize;
        // let remaining_from_end = self.buf.len() - remaining;
        // let remaining_start = self.buf_pos as usize;
        // let remaining_end = remaining_start + self.buf_len as usize;
        // self.buf
        //     .copy_within(remaining_start..remaining_end, remaining_from_end);
        // self.buf_pos = remaining as u64;
        // self.buf_len = remaining as u64;
        // println!(
        //     "remaining: {remaining:?}, bpos: {}, blen: {}",
        //     self.buf_pos, self.buf_len
        // );

        // let available_space = self.buf.len() as u64 - self.buf_pos;
        // let read_start = self.pos.saturating_sub(available_space);
        // let buf_start = remaining_from_end as u64 - (self.pos - read_start);
        // println!("space: {available_space:?}, read at: {read_start:?}, bstart: {buf_start:?}");

        // if buf_start == remaining_from_end as u64 {
        //     return Ok(false);
        // }

        // println!("{buf_start:?}..{remaining_from_end:?}");
        // // self.haystack.seek(SeekFrom::Start(read_start))?;
        // // self.haystack
        // //     .read_exact(&mut self.buf[buf_start as usize..remaining_from_end as usize])?;

        // self.pos = read_start;
        // self.buf_pos = buf_start;
        // self.buf_len = self.buf.len() as u64 - buf_start;

        // println!(
        //     "pos: {:?}, buf-pos: {:?}, buf-len: {:?}",
        //     self.pos, self.buf_pos, self.buf_len
        // );
        Ok(true)
    }

    fn rfind_in_slice(&self, data: &[u8]) -> Option<usize> {
        if let Some(ref case_insensitive_set) = self.leading_case_insentive {
            let pattern = self.finder.needle();
            let mut n = data.len();
            while n != 0 {
                let slice = &data[..n];
                let pos = slice.rfind_byteset(case_insensitive_set)?;
                // if pos + pattern.len() <= data.len() {
                // }
                // let end = candidate + self.finder.needle().len();
                // let candidate_data = &data[candidate..end];
                // if self.finder.needle().eq_ignore_ascii_case(candidate_data) {
                //     return Some(candidate);
                // } else {
                // }
            }

            None
        } else {
            self.finder.searcher.rfind(data)
        }
    }

    pub fn find_prev(&mut self) -> Option<u64> {
        None
        // let pattern = self.finder.needle();

        // loop {
        //     if self.buf_len < (pattern.len() as u64) && self.refill_buffer_rev().is_err() {
        //         return None;
        //     }

        //     let start = self.buf_pos as usize;
        //     let end = start + self.buf_len as usize;
        //     let search_slice = &self.buf[start..end];

        //     if let Some(relative_pos) = self.rfind_in_slice(search_slice) {
        //         let absolute_pos = self.pos + relative_pos as u64;
        //         self.buf_len = relative_pos as u64;

        //         return Some(absolute_pos);
        //     } else {
        //         self.buf_len = (pattern.len() - 1) as u64;
        //     }
        // }
    }
}

impl<'a, S: Source> Iterator for FinderIterRev<'a, S> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.find_prev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positions<T: Iterator<Item = u64>>(iter: T) -> Vec<u64> {
        iter.collect()
    }

    #[test]
    fn test_finder_single_match() {
        let finder = Finder::new(b"needle");
        let haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_multiple_matches() {
        let finder = Finder::new(b"an");
        let haystack = b"banana";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_finder_no_match() {
        let finder = Finder::new(b"zzz");
        let haystack = b"banana";
        let result = positions(finder.iter(haystack));
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_case_insensitive() {
        let finder = Finder::new_case_insensitive(b"NeedLe");
        let haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_match_at_end() {
        let finder = Finder::new(b"stack");
        let haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![26]);
    }

    #[test]
    fn test_finder_rev_single_match() {
        let finder = FinderRev::new(b"needle");
        let haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(haystack).unwrap());
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_multiple_matches() {
        let finder = FinderRev::new(b"an");
        let haystack = b"banana";
        let result = positions(finder.iter(haystack).unwrap());
        assert_eq!(result, vec![3, 1]); // reverse order
    }

    #[test]
    fn test_finder_rev_case_insensitive() {
        let finder = FinderRev::new_case_insensitive(b"NeEdLe");
        let haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(haystack).unwrap());
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_no_match() {
        let finder = FinderRev::new(b"zzz");
        let haystack = b"banana";
        let result = positions(finder.iter(haystack).unwrap());
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_rev_match_at_start() {
        let finder = FinderRev::new(b"find");
        let haystack = b"find the needle";
        let result = positions(finder.iter(haystack).unwrap());
        assert_eq!(result, vec![0]);
    }
}
