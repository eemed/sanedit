use crate::ByteSource;

enum Haystack<'a, T: ByteSource> {
    CaseSensitive(&'a mut T),
    CaseInsensitive(&'a mut T),
}

impl<'a, T: ByteSource> ByteSource for Haystack<'a, T> {
    fn len(&self) -> u64 {
        match self {
            Haystack::CaseSensitive(inner) => inner.len(),
            Haystack::CaseInsensitive(inner) => inner.len(),
        }
    }

    fn get(&mut self, i: u64) -> u8 {
        match self {
            Haystack::CaseSensitive(inner) => inner.get(i),
            Haystack::CaseInsensitive(inner) => {
                let mut byte = inner.get(i);
                byte.make_ascii_lowercase();
                byte
            }
        }
    }

    fn stop(&self) -> bool {
        match self {
            Haystack::CaseSensitive(inner) => inner.stop(),
            Haystack::CaseInsensitive(inner) => inner.stop(),
        }
    }

    fn as_single_chunk(&mut self) -> Option<&[u8]> {
        match self {
            Haystack::CaseSensitive(inner) => inner.as_single_chunk(),
            Haystack::CaseInsensitive(inner) => inner.as_single_chunk(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FinderRev {
    pattern: Vec<u8>,
    bad_char_table: [u64; 256],
    case_insensitive: bool,
}

impl FinderRev {
    pub fn new(pattern: &[u8]) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        let bad_char_table = Self::build_reverse_bad_char_table(pattern);
        Self {
            pattern: pattern.to_vec(),
            bad_char_table,
            case_insensitive: false,
        }
    }

    pub fn new_case_insensitive(pattern: &[u8]) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        let mut lowered = pattern.to_vec();
        lowered.make_ascii_lowercase();
        let bad_char_table = Self::build_reverse_bad_char_table(&lowered);
        Self {
            pattern: lowered,
            bad_char_table,
            case_insensitive: true,
        }
    }

    fn build_reverse_bad_char_table(pattern: &[u8]) -> [u64; 256] {
        let m = pattern.len();
        let mut table = [m as u64; 256];
        for i in (1..m).rev() {
            table[pattern[i] as usize] = i as u64;
        }
        table
    }

    pub fn iter<T: ByteSource>(&self, haystack: T) -> FinderIterRev<T> {
        let len = haystack.len();
        let m = self.pattern.len() as u64;

        FinderIterRev {
            haystack,
            finder: self,
            pos: len.saturating_sub(m),
        }
    }

    pub fn is_case_sensitive(&self) -> bool {
        !self.case_insensitive
    }
}

#[derive(Debug)]
pub struct FinderIterRev<'a, T: ByteSource> {
    haystack: T,
    finder: &'a FinderRev,
    pos: u64,
}

impl<'a, T: ByteSource> FinderIterRev<'a, T> {
    pub fn pattern_len(&self) -> usize {
        self.finder.pattern.len()
    }
}

impl<'a, T: ByteSource> Iterator for FinderIterRev<'a, T> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        fn same<B: ByteSource>(source: &mut B, pos: u64, needle: &[u8]) -> bool {
            let mut i = 0;
            while source.get(pos + i as u64) == needle[i] {
                if i == needle.len() - 1 {
                    return true;
                }

                i += 1;
            }

            false
        }

        if self.pos == u64::MAX {
            return None;
        }

        let mut haystack = if self.finder.case_insensitive {
            Haystack::CaseInsensitive(&mut self.haystack)
        } else {
            Haystack::CaseSensitive(&mut self.haystack)
        };

        let finder = self.finder;
        let needle = &finder.pattern;
        let bc = &finder.bad_char_table;

        loop {
            if same(&mut haystack, self.pos, needle) {
                let match_pos = self.pos;
                if self.pos == 0 {
                    self.pos = u64::MAX;
                } else {
                    self.pos -= 1;
                }
                return Some(match_pos);
            }
            let shift = bc[haystack.get(self.pos) as usize] as u64;
            if self.pos < shift {
                break;
            } else {
                self.pos -= shift;
            }
        }

        self.pos = u64::MAX;
        None
    }
}

#[derive(Debug, Clone)]
pub struct Finder {
    pattern: Vec<u8>,
    bad_char_table: [u64; 256],
    case_insensitive: bool,
}

impl Finder {
    pub fn new(pattern: &[u8]) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        let bad_char_table = Self::build_bad_char_table(pattern);
        Self {
            pattern: pattern.to_vec(),
            bad_char_table,
            case_insensitive: false,
        }
    }

    pub fn new_case_insensitive(pattern: &[u8]) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        let mut lowered = pattern.to_vec();
        lowered.make_ascii_lowercase();
        let bad_char_table = Self::build_bad_char_table(&lowered);
        Self {
            pattern: lowered,
            bad_char_table,
            case_insensitive: true,
        }
    }

    fn build_bad_char_table(pattern: &[u8]) -> [u64; 256] {
        let m = pattern.len();
        let mut table = [m as u64; 256];
        for i in 0..m - 1 {
            table[pattern[i] as usize] = (m - 1 - i) as u64;
        }
        table
    }

    pub fn iter<T: ByteSource>(&self, haystack: T) -> FinderIter<T> {
        FinderIter {
            haystack,
            finder: self,
            pos: 0,
        }
    }

    pub fn is_case_sensitive(&self) -> bool {
        !self.case_insensitive
    }
}

#[derive(Debug)]
pub struct FinderIter<'a, T: ByteSource> {
    haystack: T,
    finder: &'a Finder,
    pos: u64,
}

impl<'a, T: ByteSource> FinderIter<'a, T> {
    pub fn pattern_len(&self) -> usize {
        self.finder.pattern.len()
    }
}

impl<'a, T: ByteSource> Iterator for FinderIter<'a, T> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        fn same<B: ByteSource>(source: &mut B, pos: u64, needle: &[u8]) -> bool {
            let mut i = needle.len() - 1;
            while source.get(pos + i as u64) == needle[i] {
                if i == 0 {
                    return true;
                }

                i -= 1;
            }

            false
        }

        let mut haystack = if self.finder.case_insensitive {
            Haystack::CaseInsensitive(&mut self.haystack)
        } else {
            Haystack::CaseSensitive(&mut self.haystack)
        };

        let finder = self.finder;
        let needle = &finder.pattern;
        let m = needle.len() as u64;
        let n = haystack.len();
        let bc = &finder.bad_char_table;

        while n - self.pos >= m {
            if same(&mut haystack, self.pos, needle) {
                let match_pos = self.pos;
                self.pos += 1;
                return Some(match_pos);
            }
            self.pos += bc[haystack.get(self.pos + m - 1) as usize] as u64;
        }

        None
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
        let finder = Finder::new(b"ana");
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
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_multiple_matches() {
        let finder = FinderRev::new(b"ana");
        let haystack = b"banana";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![3, 1]); // reverse order
    }

    #[test]
    fn test_finder_rev_case_insensitive() {
        let finder = FinderRev::new_case_insensitive(b"NeEdLe");
        let haystack = b"find the NEEDLE in the haystack";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![9]);
    }

    #[test]
    fn test_finder_rev_no_match() {
        let finder = FinderRev::new(b"zzz");
        let haystack = b"banana";
        let result = positions(finder.iter(haystack));
        assert!(result.is_empty());
    }

    #[test]
    fn test_finder_rev_match_at_start() {
        let finder = FinderRev::new(b"find");
        let haystack = b"find the needle";
        let result = positions(finder.iter(haystack));
        assert_eq!(result, vec![0]);
    }
}
