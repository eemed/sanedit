use std::cmp::max;

use crate::ByteSource;

/// Build bad character shift table for Boyer-Moore
fn build_bad_char_table(pattern: &[u8]) -> [usize; 256] {
    let m = pattern.len();
    let mut table = [m; 256];
    for i in 0..m - 1 {
        table[pattern[i] as usize] = m - 1 - i;
    }
    table
}

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
}

#[derive(Debug, Clone)]
pub struct Finder {
    pattern: Vec<u8>,
    bad_char_table: [usize; 256],
    case_insensitive: bool,
}

impl Finder {
    pub fn new(pattern: &[u8]) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        let bad_char_table = build_bad_char_table(pattern);
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
        let bad_char_table = build_bad_char_table(&lowered);
        Self {
            pattern: lowered,
            bad_char_table,
            case_insensitive: true,
        }
    }

    pub fn iter<T: ByteSource>(&self, haystack: T) -> FinderIter<T> {
        let len = haystack.len();
        let m = self.pattern.len() as u64;

        FinderIter {
            haystack,
            finder: self,
            forward_pos: 0,
            backward_pos: len.saturating_sub(m),
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
    forward_pos: u64,
    backward_pos: u64,
}

impl<'a, T: ByteSource> FinderIter<'a, T> {
    pub fn pattern_len(&self) -> usize {
        self.finder.pattern.len()
    }
}

impl<'a, T: ByteSource> Iterator for FinderIter<'a, T> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        let mut haystack = if self.finder.case_insensitive {
            Haystack::CaseInsensitive(&mut self.haystack)
        } else {
            Haystack::CaseSensitive(&mut self.haystack)
        };

        let finder = self.finder;
        let m = finder.pattern.len() as u64;
        let n = haystack.len();

        while self.forward_pos + m <= n {
            let mut j = (m - 1) as i64;

            while j >= 0 {
                if finder.pattern[j as usize] != haystack.get(self.forward_pos + j as u64) {
                    break;
                }
                j -= 1;
            }

            if j < 0 {
                let match_pos = self.forward_pos;
                self.forward_pos += 1;
                return Some(match_pos);
            }

            let mismatched_byte = haystack.get(self.forward_pos + j as u64);
            let shift = max(1, finder.bad_char_table[mismatched_byte as usize] as u64);
            self.forward_pos += shift;
        }

        None
    }
}

impl<'a, T: ByteSource> DoubleEndedIterator for FinderIter<'a, T> {
    fn next_back(&mut self) -> Option<u64> {
        let mut haystack = if self.finder.case_insensitive {
            Haystack::CaseInsensitive(&mut self.haystack)
        } else {
            Haystack::CaseSensitive(&mut self.haystack)
        };
        let finder = self.finder;
        let m = finder.pattern.len() as u64;
        let haystack_len = haystack.len();

        while let Some(sum) = self.backward_pos.checked_add(m) {
            if sum > haystack_len {
                break;
            }

            let mut j = (m - 1) as i64;
            while j >= 0 {
                if finder.pattern[j as usize] != haystack.get(self.backward_pos + j as u64) {
                    break;
                }
                j -= 1;
            }

            if j < 0 {
                let match_pos = self.backward_pos;

                if self.backward_pos == 0 {
                    self.backward_pos = u64::MAX;
                } else {
                    self.backward_pos -= 1;
                }

                return Some(match_pos);
            }

            let mismatched_byte = haystack.get(self.backward_pos + j as u64);
            let last_occ = finder.bad_char_table[mismatched_byte as usize] as i64;
            let shift = max(1, (j as u64).saturating_sub(last_occ as u64));

            if self.backward_pos < shift {
                break;
            }

            self.backward_pos -= shift;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_case_sensitive() {
        let finder = Finder::new(b"abc");
        let haystack = b"abc abc aBc abc";

        let matches: Vec<_> = finder.iter(&haystack[..]).collect();
        assert_eq!(matches, vec![0, 4, 12]);
    }

    #[test]
    fn test_backward_case_sensitive() {
        let finder = Finder::new(b"abc");
        let haystack = b"abc abc aBc abc";

        let matches: Vec<_> = finder.iter(&haystack[..]).rev().collect();
        assert_eq!(matches, vec![12, 4, 0]);
    }

    #[test]
    fn test_forward_case_insensitive() {
        let finder = Finder::new_case_insensitive(b"abc");
        let haystack = b"ABC abc aBc abc";

        let matches: Vec<_> = finder.iter(&haystack[..]).collect();
        assert_eq!(matches, vec![0, 4, 8, 12]);
    }

    #[test]
    fn test_backward_case_insensitive() {
        let finder = Finder::new_case_insensitive(b"abc");
        let haystack = b"ABC abc aBc abc";

        let matches: Vec<_> = finder.iter(&haystack[..]).rev().collect();
        assert_eq!(matches, vec![12, 8, 4, 0]);
    }

    #[test]
    fn test_empty_haystack() {
        let finder = Finder::new(b"abc");
        let haystack = b"";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![]);
    }

    #[test]
    #[should_panic]
    fn test_empty_pattern() {
        let finder = Finder::new(b"");
    }

    #[test]
    fn test_single_byte_haystack_and_pattern_match() {
        let finder = Finder::new(b"a");
        let haystack = b"a";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![0]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![0]);
    }

    #[test]
    fn test_single_byte_haystack_and_pattern_no_match() {
        let finder = Finder::new(b"x");
        let haystack = b"a";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![]);
    }

    #[test]
    fn test_pattern_larger_than_haystack() {
        let finder = Finder::new(b"abcdef");
        let haystack = b"abc";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![]);
    }

    #[test]
    fn test_pattern_at_end_of_haystack() {
        let finder = Finder::new(b"xyz");
        let haystack = b"abcxyz";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![3]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![3]);
    }

    #[test]
    fn test_overlapping_matches() {
        let finder = Finder::new(b"aa");
        let haystack = b"aaaaa";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![0, 1, 2, 3]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![3, 2, 1, 0]);
    }

    #[test]
    fn test_full_repeat_matches() {
        let finder = Finder::new(b"ab");
        let haystack = b"abababab";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![0, 2, 4, 6]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![6, 4, 2, 0]);
    }

    #[test]
    fn test_case_insensitive_match() {
        let finder = Finder::new_case_insensitive(b"abc");
        let haystack = b"aBc AbC ABC";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![0, 4, 8]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![8, 4, 0]);
    }

    #[test]
    fn test_case_sensitive_no_match_when_case_differs() {
        let finder = Finder::new(b"abc");
        let haystack = b"aBc AbC ABC";

        let matches: Vec<_> = finder.iter(haystack).collect();
        assert_eq!(matches, vec![]);

        let matches_rev: Vec<_> = finder.iter(haystack).rev().collect();
        assert_eq!(matches_rev, vec![]);
    }
}
