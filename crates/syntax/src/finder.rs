use std::io::{self, Read, Seek, SeekFrom};

use memchr::{memchr, memchr2};

#[derive(Debug, Clone)]
pub struct Finder {
    pattern: Vec<u8>,
    case_insensitive: bool,
}

impl Finder {
    pub fn new(pattern: &[u8]) -> Self {
        Finder {
            pattern: pattern.into(),
            case_insensitive: false,
        }
    }

    pub fn new_case_insensitive(pattern: &[u8]) -> Self {
        Finder {
            pattern: pattern.to_ascii_lowercase(),
            case_insensitive: true,
        }
    }

    pub fn iter<R: Read>(&self, haystack: R) -> FinderIter<'_, R> {
        FinderIter {
            haystack,
            buf: vec![0u8; 128 * 1024].into_boxed_slice(),
            buf_pos: 0,
            buf_len: 0,
            finder: self,
            pos: 0,
        }
    }

    pub fn is_case_sensitive(&self) -> bool {
        !self.case_insensitive
    }

    pub fn pattern(&self) -> &[u8] {
        &self.pattern
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
}

impl<'a, R: Read> FinderIter<'a, R> {
    fn refill_buffer(&mut self) -> io::Result<bool> {
        let remaining = self.buf_len - self.buf_pos;
        self.buf
            .copy_within(self.buf_pos as usize..self.buf_len as usize, 0);
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

    fn find_case_sensitive(&self, data: &[u8]) -> Option<usize> {
        let pattern = &self.finder.pattern;
        let first_byte = pattern[0];
        let mut pos = 0;

        while pos <= data.len().saturating_sub(pattern.len()) {
            match memchr(first_byte, &data[pos..]) {
                Some(offset) => {
                    let candidate_pos = pos + offset;

                    if candidate_pos + pattern.len() <= data.len()
                        && &data[candidate_pos..candidate_pos + pattern.len()] == pattern
                    {
                        return Some(candidate_pos);
                    }

                    pos = candidate_pos + 1;
                }
                None => break,
            }
        }

        None
    }

    fn find_case_insensitive(&self, data: &[u8]) -> Option<usize> {
        let pattern = &self.finder.pattern;
        let lower = pattern[0].to_ascii_lowercase();
        let upper = pattern[0].to_ascii_uppercase();

        let mut pos = 0;

        while pos <= data.len().saturating_sub(pattern.len()) {
            match memchr2(lower, upper, &data[pos..]) {
                Some(offset) => {
                    let candidate_pos = pos + offset;

                    if self.matches_case_insensitive(&data[candidate_pos..]) {
                        return Some(candidate_pos);
                    }

                    pos = candidate_pos + 1;
                }
                None => break,
            }
        }

        None
    }

    fn matches_case_insensitive(&self, data: &[u8]) -> bool {
        let pattern = &self.finder.pattern;
        if data.len() < pattern.len() {
            return false;
        }

        let pattern_data = &data[..pattern.len()].to_ascii_lowercase();
        pattern_data == pattern
    }

    fn find_in_slice(&self, data: &[u8]) -> Option<usize> {
        if self.finder.case_insensitive {
            self.find_case_insensitive(data)
        } else {
            self.find_case_sensitive(data)
        }
    }

    pub fn find_next(&mut self) -> io::Result<Option<u64>> {
        let pattern = &self.finder.pattern;
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
    pattern: Vec<u8>,
    case_insensitive: bool,
}

impl FinderRev {
    pub fn new(pattern: &[u8]) -> Self {
        FinderRev {
            pattern: pattern.into(),
            case_insensitive: false,
        }
    }

    pub fn new_case_insensitive(pattern: &[u8]) -> Self {
        FinderRev {
            pattern: pattern.to_ascii_lowercase(),
            case_insensitive: true,
        }
    }

    pub fn iter<R: Read + Seek>(&self, haystack: R) -> io::Result<FinderIterRev<'_, R>> {
        FinderIterRev::new(haystack, self)
    }

    pub fn is_case_sensitive(&self) -> bool {
        !self.case_insensitive
    }

    pub fn pattern(&self) -> &[u8] {
        &self.pattern
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
}

impl<'a, R: Read + Seek> FinderIterRev<'a, R> {
    fn new(mut haystack: R, finder: &'a FinderRev) -> io::Result<Self> {
        let file_size = haystack.seek(SeekFrom::End(0))?;

        let mut iter = Self {
            haystack,
            buf: vec![0u8; 128 * 1024].into_boxed_slice(),
            buf_pos: 0,
            buf_len: 0,
            finder,
            pos: file_size,
            file_size,
        };

        iter.refill_buffer_rev()?;
        Ok(iter)
    }

    fn refill_buffer_rev(&mut self) -> io::Result<bool> {
        if self.pos == 0 {
            self.buf_len = 0;
            return Ok(false);
        }

        let buf_size = self.buf.len() as u64;
        let read_start = if self.pos > buf_size {
            self.pos - buf_size
        } else {
            0
        };

        let read_size = (self.pos - read_start) as usize;

        self.haystack.seek(SeekFrom::Start(read_start))?;
        self.haystack.read_exact(&mut self.buf[..read_size])?;

        self.buf_pos = 0;
        self.buf_len = read_size as u64;
        self.pos = read_start;

        Ok(true)
    }

    fn rfind_case_sensitive(&self, data: &[u8]) -> Option<usize> {
        let pattern = &self.finder.pattern;
        if pattern.is_empty() {
            return Some(data.len());
        }

        let last_byte = pattern[pattern.len() - 1];
        let mut pos = data.len();

        while pos >= pattern.len() {
            let search_range = if pos > 0 { &data[..pos] } else { &[] };

            match memchr(last_byte, search_range) {
                Some(offset) => {
                    let candidate_end = offset + 1;
                    if candidate_end >= pattern.len() {
                        let candidate_start = candidate_end - pattern.len();
                        if &data[candidate_start..candidate_end] == pattern {
                            return Some(candidate_start);
                        }
                    }
                    pos = offset;
                }
                None => break,
            }

            if pos == 0 {
                break;
            }
            pos -= 1;
        }

        None
    }

    fn rfind_case_insensitive(&self, data: &[u8]) -> Option<usize> {
        let pattern = &self.finder.pattern;
        if pattern.is_empty() {
            return Some(data.len());
        }

        let last_byte_lower = pattern[pattern.len() - 1].to_ascii_lowercase();
        let last_byte_upper = pattern[pattern.len() - 1].to_ascii_uppercase();

        let mut pos = data.len();

        while pos >= pattern.len() {
            let search_range = if pos > 0 { &data[..pos] } else { &[] };

            match memchr2(last_byte_lower, last_byte_upper, search_range) {
                Some(offset) => {
                    let candidate_end = offset + 1;
                    if candidate_end >= pattern.len() {
                        let candidate_start = candidate_end - pattern.len();
                        if self.matches_case_insensitive(&data[candidate_start..candidate_end]) {
                            return Some(candidate_start);
                        }
                    }
                    pos = offset;
                }
                None => break,
            }

            if pos == 0 {
                break;
            }
            pos -= 1;
        }

        None
    }

    fn matches_case_insensitive(&self, data: &[u8]) -> bool {
        let pattern = &self.finder.pattern;
        if data.len() < pattern.len() {
            return false;
        }

        let pattern_data = &data[..pattern.len()].to_ascii_lowercase();
        pattern_data == pattern
    }

    fn rfind_in_slice(&self, data: &[u8]) -> Option<usize> {
        if self.finder.case_insensitive {
            self.rfind_case_insensitive(data)
        } else {
            self.rfind_case_sensitive(data)
        }
    }

    pub fn find_prev(&mut self) -> io::Result<Option<u64>> {
        let pattern = &self.finder.pattern;

        loop {
            if self.buf_len == 0 {
                return Ok(None);
            }

            let search_slice = &self.buf[self.buf_pos as usize..self.buf_len as usize];

            if let Some(relative_pos) = self.rfind_in_slice(search_slice) {
                let absolute_pos = self.pos + relative_pos as u64;

                // Move buffer position past this match for next search
                self.buf_pos = (relative_pos + pattern.len()) as u64;
                self.pos = self.pos + self.buf_pos;

                return Ok(Some(absolute_pos));
            } else {
                // No match in current buffer, load previous chunk
                if !self.refill_buffer_rev()? {
                    return Ok(None);
                }
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
        let finder = FinderRev::new(b"ana");
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
