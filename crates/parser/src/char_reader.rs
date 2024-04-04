use std::{iter::Rev, str::Chars};

pub trait CharReader {
    type I: Iterator<Item = char>;
    type O: Iterator<Item = char>;

    /// Length of all the bytes in this reader utf8
    fn len(&self) -> usize;

    /// Wether to stop parsing and return an error
    fn stop(&self) -> bool;

    /// Reverse chars
    fn chars_rev(&self) -> Self::I;

    fn chars_at(&self, at: usize) -> Self::O;

    fn matches(&self, at: usize, exp: &str) -> bool {
        let mut chars = self.chars_at(at);
        for e in exp.chars() {
            if Some(e) != chars.next() {
                return false;
            }
        }

        true
    }
}

impl<'a> CharReader for &'a str {
    type I = Rev<Chars<'a>>;
    type O = Chars<'a>;

    fn len(&self) -> usize {
        self.as_bytes().len()
    }

    fn stop(&self) -> bool {
        false
    }

    fn chars_rev(&self) -> Self::I {
        self.chars().rev()
    }

    fn chars_at(&self, at: usize) -> Self::O {
        self[at..].chars()
    }
}
