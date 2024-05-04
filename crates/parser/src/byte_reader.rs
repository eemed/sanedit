use std::{cmp::min, ops::Range};

use sanedit_buffer::utf8::decode_utf8_iter;

pub trait ByteReader {
    type I: Iterator<Item = u8>;

    /// Length of all the bytes in this reader utf8
    fn len(&self) -> usize;

    /// Wether to stop parsing and return an error
    fn stop(&self) -> bool;

    fn iter(&self, range: Range<usize>) -> Self::I;

    fn at(&self, at: usize) -> u8 {
        self.iter(at..at + 1).next().unwrap()
    }

    fn matches(&self, at: usize, exp: &[u8]) -> bool {
        let max = min(at + exp.len(), self.len());
        let mut bytes = self.iter(at..max);
        for e in exp {
            match bytes.next() {
                Some(ch) => {
                    if ch != *e {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }

    fn char_between(&self, at: usize, start: char, end: char) -> Option<usize> {
        let max = min(at + 4, self.len());
        let bytes = self.iter(at..max);
        let (ch, size) = decode_utf8_iter(bytes);
        let ch = ch?;

        if start <= ch && ch <= end {
            Some(size)
        } else {
            None
        }
    }
}

impl<'a> ByteReader for &'a str {
    type I = std::iter::Copied<std::slice::Iter<'a, u8>>;

    fn len(&self) -> usize {
        self.as_bytes().len()
    }

    fn stop(&self) -> bool {
        false
    }

    fn iter(&self, range: Range<usize>) -> Self::I {
        self.as_bytes()[range].iter().copied()
    }
}
