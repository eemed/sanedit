use std::ops::Range;

use crate::piece_tree::{Bytes, PieceTree};

const REPLACEMENT_CHAR: char = '\u{FFFD}';

#[derive(Debug, Clone)]
pub struct Chars<'a> {
    bytes: Bytes<'a>,
    buf: [u8; 4],
    valid_to: usize,
    invalid: bool,
}

impl<'a> Chars<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTree, at: usize) -> Chars<'a> {
        let bytes = Bytes::new(pt, at);
        Chars {
            bytes,
            buf: [0; 4],
            valid_to: 0,
            invalid: false,
        }
    }

    #[inline]
    pub(crate) fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Chars<'a> {
        debug_assert!(
            range.end - range.start >= at,
            "Attempting to index {} over slice len {} ",
            at,
            range.end - range.start,
        );
        let bytes = Bytes::new_from_slice(pt, at, range);
        Chars {
            bytes,
            buf: [0; 4],
            valid_to: 0,
            invalid: false,
        }
    }

    #[inline(always)]
    fn read_next_byte(&mut self) -> Option<()> {
        let byte = self.bytes.next()?;
        self.buf[self.valid_to] = byte;
        self.valid_to += 1;
        Some(())
    }

    pub fn next(&mut self) -> Option<(usize, char)> {
        let mut start = self.bytes.pos();

        loop {
            match decode_char(&self.buf[..self.valid_to]) {
                DecodeResult::Invalid => {
                    if self.invalid {
                        self.valid_to = 0;
                        start = self.bytes.pos();
                        self.read_next_byte()?;
                    } else {
                        self.invalid = true;
                        return Some((start, REPLACEMENT_CHAR));
                    }
                }
                DecodeResult::Incomplete => {
                    if self.read_next_byte().is_none() {
                        self.invalid = true;
                        return Some((self.bytes.pos(), REPLACEMENT_CHAR));
                    }
                }
                DecodeResult::Ok(ch) => {
                    self.invalid = false;
                    self.valid_to = 0;
                    return Some((start, ch));
                }
            }
        }
    }

    #[inline(always)]
    fn read_prev_until_leading_utf8_byte(&mut self) -> Option<bool> {
        while self.valid_to != 4 {
            let byte = self.bytes.prev()?;
            self.buf[self.buf.len() - self.valid_to - 1] = byte;
            self.valid_to += 1;

            if is_leading_utf8_byte(byte) {
                return Some(true);
            }
        }

        return Some(false);
    }

    // TODO yield replacement chars at the same positions as next
    pub fn prev(&mut self) -> Option<(usize, char)> {
        loop {
            match decode_char(&self.buf[self.buf.len() - self.valid_to..]) {
                DecodeResult::Invalid => {
                    if self.invalid {
                        self.valid_to = 0;
                        while !self.read_prev_until_leading_utf8_byte()? {}
                    } else {
                        self.invalid = true;
                        return Some((self.bytes.pos(), REPLACEMENT_CHAR));
                    }
                }
                DecodeResult::Incomplete => {
                    if self.read_prev_until_leading_utf8_byte().is_none() {
                        self.invalid = true;
                        return Some((self.bytes.pos(), REPLACEMENT_CHAR));
                    }
                }
                DecodeResult::Ok(ch) => {
                    self.invalid = false;
                    self.valid_to = 0;
                    return Some((self.bytes.pos(), ch));
                }
            }
        }
    }
}

#[derive(Debug)]
enum DecodeResult {
    Invalid,
    Incomplete,
    Ok(char),
}

#[inline]
fn decode_char(bytes: &[u8]) -> DecodeResult {
    if bytes.is_empty() {
        return DecodeResult::Incomplete;
    }

    match std::str::from_utf8(bytes) {
        Ok(c) => {
            let ch = c.chars().next().unwrap();
            DecodeResult::Ok(ch)
        }
        Err(e) => {
            if e.error_len().is_some() && e.valid_up_to() == 0 {
                DecodeResult::Invalid
            } else {
                DecodeResult::Incomplete
            }
        }
    }
}

#[inline(always)]
fn is_leading_utf8_byte(byte: u8) -> bool {
    (byte & 0xC0) != 0x80
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn next() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"ab\xFF\xFF\xFF\xFF\xFFba");

        let mut chars = pt.chars();
        assert_eq!(Some((0, 'a')), chars.next());
        assert_eq!(Some((1, 'b')), chars.next());
        assert_eq!(Some((2, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((7, 'b')), chars.next());
        assert_eq!(Some((8, 'a')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"ab\xFF\xFF\xFF\xFF\xFFba");
        let mut chars = pt.chars_at(pt.len());

        assert_eq!(Some((8, 'a')), chars.prev());
        assert_eq!(Some((7, 'b')), chars.prev());
        assert_eq!(Some((6, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((1, 'b')), chars.prev());
        assert_eq!(Some((0, 'a')), chars.prev());
        assert_eq!(None, chars.prev());
    }

    #[test]
    fn next_then_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"ab\xFF\xFF\xFF\xFF\xFFba");

        let mut chars = pt.chars();
        assert_eq!(Some((0, 'a')), chars.next());
        assert_eq!(Some((1, 'b')), chars.next());
        assert_eq!(Some((2, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((7, 'b')), chars.next());
        assert_eq!(Some((7, 'b')), chars.prev());
        assert_eq!(Some((6, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((1, 'b')), chars.prev());
        assert_eq!(Some((0, 'a')), chars.prev());
        assert_eq!(None, chars.prev());
    }

    #[test]
    fn middle_of_char() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "§ab");

        let slice = pt.slice(1..);
        let mut chars = slice.chars();
        assert_eq!(Some((0, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((1, 'a')), chars.next());
        assert_eq!(Some((2, 'b')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);
        let mut chars = pt.chars();

        assert_eq!(Some((0, '❤')), chars.next());
        assert_eq!(Some((3, '🤍')), chars.next());
        assert_eq!(Some((7, '🥳')), chars.next());
        assert_eq!(Some((11, '❤')), chars.next());
        assert_eq!(Some((14, '\u{fe0f}')), chars.next());
        assert_eq!(Some((17, '간')), chars.next());
        assert_eq!(Some((20, '÷')), chars.next());
        assert_eq!(Some((22, '나')), chars.next());
        assert_eq!(Some((25, '는')), chars.next());
        assert_eq!(Some((28, '산')), chars.next());
        assert_eq!(Some((31, '다')), chars.next());
        assert_eq!(Some((34, '⛄')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn multi_byte_slice() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);
        let slice = pt.slice(5..20);
        let mut chars = slice.chars();

        assert_eq!(Some((0, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((2, '🥳')), chars.next());
        assert_eq!(Some((6, '❤')), chars.next());
        assert_eq!(Some((9, '\u{fe0f}')), chars.next());
        assert_eq!(Some((12, '간')), chars.next());
        assert_eq!(Some((15, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn multi_byte_slice_prev() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);
        let slice = pt.slice(5..20);
        let mut chars = slice.chars_at(slice.len());

        assert_eq!(Some((15, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((12, '간')), chars.prev());
        assert_eq!(Some((9, '\u{fe0f}')), chars.prev());
        assert_eq!(Some((6, '❤')), chars.prev());
        assert_eq!(Some((2, '🥳')), chars.prev());
        assert_eq!(Some((0, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(None, chars.prev());
    }
}
