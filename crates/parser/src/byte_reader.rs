use std::{cmp::min, ops::Range};

use sanedit_buffer::utf8::decode_utf8_iter;

pub trait ByteReader {
    type I: Iterator<Item = u8>;

    /// Length of all the bytes in this reader utf8
    fn len(&self) -> u64;

    /// Wether to stop parsing and return an error
    fn stop(&self) -> bool;

    fn iter(&self, range: Range<u64>) -> Self::I;

    fn at(&self, at: u64) -> u8 {
        self.iter(at..at + 1).next().unwrap()
    }

    fn matches(&self, at: u64, exp: &[u8]) -> bool {
        let max = min(at + exp.len() as u64, self.len());
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

    fn char_between(&self, at: u64, start: char, end: char) -> Option<u64> {
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

    fn len(&self) -> u64 {
        self.as_bytes().len() as u64
    }

    fn stop(&self) -> bool {
        false
    }

    fn iter(&self, range: Range<u64>) -> Self::I {
        self.as_bytes()[range.start as usize..range.end as usize]
            .iter()
            .copied()
    }
}
