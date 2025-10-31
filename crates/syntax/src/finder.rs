use bstr::ByteSlice;
use std::{
    cmp::max,
    io::{self, Read, Seek, SeekFrom},
};

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

    pub fn iter<R: Read>(&self, haystack: R) -> FinderIter<'_, R> {
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
            buf: vec![0u8; max(BUF_SIZE, self.needle().len() * 2)].into_boxed_slice(),
            buf_pos: 0,
            buf_len: 0,
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
pub struct FinderIter<'a, R: Read> {
    haystack: R,
    buf: Box<[u8]>,
    buf_pos: u64,
    buf_len: u64,
    finder: &'a Finder,
    pos: u64,
    leading_case_insentive: Option<[u8; 2]>,
}

impl<'a, R: Read> FinderIter<'a, R> {
    fn refill_buffer(&mut self) -> io::Result<bool> {
        self.buf
            .copy_within(self.buf_pos as usize..self.buf_len as usize, 0);
        let remaining = self.buf_len - self.buf_pos;
        self.buf_pos = 0;
        self.buf_len = remaining;
        let mut did_read = false;

        while self.buf_len < self.buf.len() as u64 {
            let bytes_read = self.haystack.read(&mut self.buf[self.buf_len as usize..])?;
            self.buf_len += bytes_read as u64;
            if bytes_read == 0 {
                break;
            }
            did_read = true;
        }

        Ok(did_read)
    }

    fn find_in_slice(&self, data: &[u8]) -> Option<usize> {
        if let Some(ref case_insensitive_set) = self.leading_case_insentive {
            loop {
                let candidate = data.find_byteset(case_insensitive_set)?;
                let end = candidate + self.finder.needle().len();
                if end > data.len() {
                    return None;
                }

                let candidate_data = &data[candidate..end];
                if self.finder.needle().eq_ignore_ascii_case(candidate_data) {
                    return Some(candidate);
                }
            }
        } else {
            self.finder.searcher.find(data)
        }
    }

    pub fn find_next(&mut self) -> io::Result<Option<u64>> {
        let pattern = self.finder.needle();
        loop {
            let remaining = (self.buf_len - self.buf_pos) as usize;
            if remaining < pattern.len() && !self.refill_buffer()? {
                return Ok(None);
            }

            let search_slice = &self.buf[self.buf_pos as usize..self.buf_len as usize];

            if let Some(relative_pos) = self.find_in_slice(search_slice) {
                let absolute_pos = self.pos + relative_pos as u64;

                let advance_by = (relative_pos + pattern.len()) as u64;
                self.buf_pos += advance_by;
                self.pos += advance_by;

                return Ok(Some(absolute_pos));
            } else {
                let advance_by = search_slice
                    .len()
                    .saturating_sub(pattern.len().saturating_sub(1))
                    as u64;
                self.buf_pos += advance_by;
                self.pos += advance_by;
            }
        }
    }
}

impl<'a, R: Read> Iterator for FinderIter<'a, R> {
    type Item = io::Result<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.find_next() {
            Ok(Some(pos)) => Some(Ok(pos)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
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

    pub fn iter<R: Read + Seek>(&self, haystack: R) -> io::Result<FinderIterRev<'_, R>> {
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
pub struct FinderIterRev<'a, R: Read + Seek> {
    haystack: R,
    buf: Box<[u8]>,
    buf_pos: u64,
    buf_len: u64,
    finder: &'a FinderRev,
    pos: u64,
    file_size: u64,
    leading_case_insentive: Option<[u8; 2]>,
}

impl<'a, R: Read + Seek> FinderIterRev<'a, R> {
    fn new(mut haystack: R, finder: &'a FinderRev) -> io::Result<Self> {
        let file_size = haystack.seek(SeekFrom::End(0))?;

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
            buf: vec![0u8; max(BUF_SIZE, finder.needle().len() * 2)].into_boxed_slice(),
            buf_pos: 0,
            buf_len: 0,
            finder,
            pos: file_size,
            file_size,
            leading_case_insentive,
        })
    }

    fn refill_buffer_rev(&mut self) -> io::Result<bool> {
        let remaining = self.buf_len as usize;
        let remaining_from_end = self.buf.len() - remaining;
        let remaining_start = self.buf_pos as usize;
        let remaining_end = remaining_start + self.buf_len as usize;
        self.buf
            .copy_within(remaining_start..remaining_end, remaining_from_end);
        self.buf_pos = remaining as u64;
        self.buf_len = remaining as u64;
        println!(
            "remaining: {remaining:?}, bpos: {}, blen: {}",
            self.buf_pos, self.buf_len
        );

        let available_space = self.buf.len() as u64 - self.buf_pos;
        let read_start = self.pos.saturating_sub(available_space);
        let buf_start = remaining_from_end as u64 - (self.pos - read_start);
        println!("space: {available_space:?}, read at: {read_start:?}, bstart: {buf_start:?}");

        if buf_start == remaining_from_end as u64 {
            return Ok(false);
        }

        println!("{buf_start:?}..{remaining_from_end:?}");
        self.haystack.seek(SeekFrom::Start(read_start))?;
        self.haystack
            .read_exact(&mut self.buf[buf_start as usize..remaining_from_end as usize])?;

        self.pos = read_start;
        self.buf_pos = buf_start;
        self.buf_len = self.buf.len() as u64 - buf_start;

        println!(
            "pos: {:?}, buf-pos: {:?}, buf-len: {:?}",
            self.pos, self.buf_pos, self.buf_len
        );
        Ok(true)
    }

    fn rfind_in_slice(&self, data: &[u8]) -> Option<usize> {
        if let Some(ref case_insensitive_set) = self.leading_case_insentive {
            loop {
                let candidate = data.rfind_byteset(case_insensitive_set)?;
                let end = candidate + self.finder.needle().len();
                let candidate_data = &data[candidate..end];
                if self.finder.needle().eq_ignore_ascii_case(candidate_data) {
                    return Some(candidate);
                }
            }
        } else {
            self.finder.searcher.rfind(data)
        }
    }

    pub fn find_prev(&mut self) -> io::Result<Option<u64>> {
        let pattern = self.finder.needle();

        loop {
            if self.buf_len < (pattern.len() as u64) && !self.refill_buffer_rev()? {
                return Ok(None);
            }

            let start = self.buf_pos as usize;
            let end = start + self.buf_len as usize;
            let search_slice = &self.buf[start..end];

            if let Some(relative_pos) = self.rfind_in_slice(search_slice) {
                let absolute_pos = self.pos + relative_pos as u64;
                self.buf_len = relative_pos as u64;

                return Ok(Some(absolute_pos));
            } else {
                self.buf_len = (pattern.len() - 1) as u64;
            }
        }
    }
}

impl<'a, R: Read + Seek> Iterator for FinderIterRev<'a, R> {
    type Item = io::Result<u64>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.find_prev() {
            Ok(Some(pos)) => Some(Ok(pos)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positions<T: Iterator<Item = io::Result<u64>>>(iter: T) -> Vec<u64> {
        iter.map(|k| k.unwrap()).collect()
    }

    #[test]
    fn test_finder_single_match() {
        let finder = Finder::new(b"needle");
        let haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_multiple_matches() {
        let finder = Finder::new(b"an");
        let haystack = b"banana";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)));
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_finder_no_match() {
        let finder = Finder::new(b"zzz");
        let haystack = b"banana";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)));
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_case_insensitive() {
        let finder = Finder::new_case_insensitive(b"NeedLe");
        let haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_match_at_end() {
        let finder = Finder::new(b"stack");
        let haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)));
        assert_eq!(result, vec![26]);
    }

    #[test]
    fn test_finder_rev_single_match() {
        let finder = FinderRev::new(b"needle");
        let haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)).unwrap());
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_multiple_matches() {
        let finder = FinderRev::new(b"an");
        let haystack = b"banana";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)).unwrap());
        assert_eq!(result, vec![3, 1]); // reverse order
    }

    #[test]
    fn test_finder_rev_case_insensitive() {
        let finder = FinderRev::new_case_insensitive(b"NeEdLe");
        let haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)).unwrap());
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_no_match() {
        let finder = FinderRev::new(b"zzz");
        let haystack = b"banana";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)).unwrap());
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_rev_match_at_start() {
        let finder = FinderRev::new(b"find");
        let haystack = b"find the needle";
        let result = positions(finder.iter(std::io::Cursor::new(haystack)).unwrap());
        assert_eq!(result, vec![0]);
    }
}
