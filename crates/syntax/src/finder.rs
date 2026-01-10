use bstr::ByteSlice;

use crate::source::Source;

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

    pub fn iter<'a, 'b, S: Source>(&'a self, haystack: &'b mut S) -> FinderIter<'a, 'b, S> {
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
pub struct FinderIter<'a, 'b, S: Source> {
    haystack: &'b mut S,
    /// Relative current buffer pos
    pos: usize,
    finder: &'a Finder,
    leading_case_insentive: Option<[u8; 2]>,
}

impl<'a, 'b, S: Source> FinderIter<'a, 'b, S> {
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

    fn find_next(&mut self) -> Option<u64> {
        let pattern = self.finder.needle();
        if self.haystack.stop() {
            return None;
        }

        loop {
            let (buf_pos, buf) = self.haystack.buffer();
            let remaining = buf.len() - self.pos;
            if remaining < pattern.len() {
                if !self.haystack.stop()
                    && self
                        .haystack
                        .refill_buffer(buf_pos + self.pos as u64)
                        .ok()?
                {
                    self.pos = 0;
                } else {
                    return None;
                }
            }

            let (buf_pos, buf) = self.haystack.buffer();
            let search_slice = &buf[self.pos..];

            if let Some(relative_pos) = self.find_in_slice(search_slice) {
                let absolute_pos = buf_pos + self.pos as u64 + relative_pos as u64;
                self.pos += relative_pos + pattern.len();
                return Some(absolute_pos);
            } else {
                self.pos += search_slice.len().saturating_sub(pattern.len() - 1);
            }
        }
    }

    pub fn needle(&self) -> &[u8] {
        self.finder.needle()
    }
}

impl<'a, 'b, S: Source> Iterator for FinderIter<'a, 'b, S> {
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

    pub fn iter<'b, S: Source>(&self, haystack: &'b mut S) -> FinderIterRev<'_, 'b, S> {
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
pub struct FinderIterRev<'a, 'b, S: Source> {
    haystack: &'b mut S,
    finder: &'a FinderRev,
    /// Relative position to current buffer
    pos: usize,
    leading_case_insentive: Option<[u8; 2]>,
}

impl<'a, 'b, S: Source> FinderIterRev<'a, 'b, S> {
    fn new(haystack: &'b mut S, finder: &'a FinderRev) -> Self {
        let leading_case_insentive = if finder.is_case_sensitive() {
            None
        } else {
            let mut upper = finder.needle()[0];
            upper.make_ascii_uppercase();

            let mut lower = finder.needle()[0];
            lower.make_ascii_lowercase();
            Some([lower, upper])
        };
        let _ = haystack.refill_buffer_rev(haystack.len());
        let (_, buf) = haystack.buffer();

        Self {
            finder,
            pos: buf.len(),
            leading_case_insentive,
            haystack,
        }
    }

    fn rfind_in_slice(&self, data: &[u8]) -> Option<usize> {
        if let Some(ref case_insensitive_set) = self.leading_case_insentive {
            let pattern = self.finder.needle();
            let mut n = data.len().saturating_sub(pattern.len().saturating_sub(1));
            loop {
                let slice = &data[..n];
                n = slice.rfind_byteset(case_insensitive_set)?;
                let end = n + pattern.len();
                if pattern.eq_ignore_ascii_case(&data[n..end]) {
                    return Some(n);
                }
            }
        } else {
            self.finder.searcher.rfind(data)
        }
    }

    fn find_prev(&mut self) -> Option<u64> {
        let pattern = self.finder.needle();

        if self.haystack.stop() {
            return None;
        }

        loop {
            let (buf_pos, _) = self.haystack.buffer();
            if self.pos < pattern.len() {
                if !self.haystack.stop()
                    && self
                        .haystack
                        .refill_buffer_rev(buf_pos + self.pos as u64)
                        .ok()?
                {
                    let (_, buf) = self.haystack.buffer();
                    self.pos = buf.len();
                } else {
                    return None;
                }
            }

            let (buf_pos, buf) = self.haystack.buffer();
            let search_slice = &buf[..self.pos];

            if let Some(relative_pos) = self.rfind_in_slice(search_slice) {
                self.pos = relative_pos;
                let absolute_pos = buf_pos + relative_pos as u64;
                return Some(absolute_pos);
            } else {
                self.pos = pattern.len() - 1;
            }
        }
    }

    pub fn needle(&self) -> &[u8] {
        self.finder.needle()
    }
}

impl<'a, 'b, S: Source> Iterator for FinderIterRev<'a, 'b, S> {
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
        let mut haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_multiple_matches() {
        let finder = Finder::new(b"an");
        let mut haystack = b"banana";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_finder_no_match() {
        let finder = Finder::new(b"zzz");
        let mut haystack = b"banana";
        let result = positions(finder.iter(&mut haystack));
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_case_insensitive() {
        let finder = Finder::new_case_insensitive(b"NeedLe");
        let mut haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_match_at_end() {
        let finder = Finder::new(b"stack");
        let mut haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![26]);
    }

    #[test]
    fn test_finder_rev_single_match() {
        let finder = FinderRev::new(b"needle");
        let mut haystack = b"find the needle in the haystack";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_multiple_matches() {
        let finder = FinderRev::new(b"an");
        let mut haystack = b"banana";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![3, 1]); // reverse order
    }

    #[test]
    fn test_finder_rev_case_insensitive() {
        let finder = FinderRev::new_case_insensitive(b"NeEdLe");
        let mut haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_no_match() {
        let finder = FinderRev::new(b"zzz");
        let mut haystack = b"banana";
        let result = positions(finder.iter(&mut haystack));
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_rev_match_at_start() {
        let finder = FinderRev::new(b"find");
        let mut haystack = b"find the needle";
        let result = positions(finder.iter(&mut haystack));
        assert_eq!(result, vec![0]);
    }
}
