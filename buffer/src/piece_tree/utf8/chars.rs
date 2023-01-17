use std::{cmp, ops::Range};

use crate::piece_tree::{Bytes, PieceTree};

const REPLACEMENT_CHAR: char = '\u{FFFD}';

/// Iterate over the chars of the buffer.
/// If invalid UTF8 is encountered replace them with the replacement character.
/// The strategy used is ["maximal subpart" strategy](https://www.unicode.org/review/pr-121.html).
/// Basically every codepoint (1-4) bytes is replaced with one replacement
/// character. If for example the first 3 bytes are valid but then the 4th is
/// invalid, the 3 valid bytes will be replaced with the replacement character.
#[derive(Debug, Clone)]
pub struct Chars<'a> {
    bytes: Bytes<'a>,
}

impl<'a> Chars<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTree, at: usize) -> Chars<'a> {
        let bytes = Bytes::new(pt, at);
        Chars { bytes }
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
        Chars { bytes }
    }

    pub fn next(&mut self) -> Option<(usize, usize, char)> {
        let start = self.bytes.pos();
        let mut buf = [0u8; 4];
        let mut read = 0;

        loop {
            match decode_char(&buf[..read]) {
                DecodeResult::Invalid(valid_up_to) => {
                    // TODO is there better way than to scroll back.
                    // Maybe store this in a buf?
                    for _ in 0..read - valid_up_to {
                        self.bytes.prev();
                    }
                    return Some((start, start + valid_up_to, REPLACEMENT_CHAR));
                }
                DecodeResult::Incomplete => match self.bytes.next() {
                    Some(byte) => {
                        buf[read] = byte;
                        read += 1;
                    }
                    None => {
                        if read == 0 {
                            return None;
                        }

                        return Some((start, start + read, REPLACEMENT_CHAR));
                    }
                },
                DecodeResult::Ok(ch) => return Some((start, start + read, ch)),
            }
        }
    }

    // #[inline(always)]
    // fn read_prev_until_leading_utf8_byte(&mut self) -> Option<bool> {
    //     while self.valid_to != 4 {
    //         let byte = self.bytes.prev()?;
    //         self.buf[self.buf.len() - self.valid_to - 1] = byte;
    //         self.valid_to += 1;

    //         if is_leading_utf8_byte(byte) {
    //             return Some(true);
    //         }
    //     }

    //     return Some(false);
    // }

    pub fn prev(&mut self) -> Option<(usize, char)> {
        let end = self.bytes.pos();
        let mut buf = [0u8; 4];
        let mut read = 0;

        // Read to buf until is leading byte, or invalid byte, or 4 bytes total
        // read.

        todo!()
        // self.valid_to = 0;
        // loop {
        //     match decode_char(&self.buf[self.buf.len() - self.valid_to..]) {
        //         DecodeResult::Invalid => {
        //             if self.invalid {
        //                 self.valid_to = 0;
        //                 while !self.read_prev_until_leading_utf8_byte()? {}
        //             } else {
        //                 self.invalid = true;
        //                 return Some((self.bytes.pos(), REPLACEMENT_CHAR));
        //             }
        //         }
        //         DecodeResult::Incomplete => {
        //             if self.read_prev_until_leading_utf8_byte().is_none() {
        //                 if self.valid_to == 0 {
        //                     return None;
        //                 }
        //                 return Some((self.bytes.pos(), REPLACEMENT_CHAR));
        //             }
        //         }
        //         DecodeResult::Ok(ch) => {
        //             self.invalid = false;
        //             self.valid_to = 0;
        //             return Some((self.bytes.pos(), ch));
        //         }
        //     }
        // }
    }
}

#[derive(Debug)]
enum DecodeResult {
    Invalid(usize),
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
        Err(e) => match e.error_len() {
            Some(n) => DecodeResult::Invalid(n),
            None => DecodeResult::Incomplete,
        },
    }
}

#[inline(always)]
fn is_leading_utf8_byte(byte: u8) -> bool {
    (byte & 0xC0) != 0x80
}

#[inline(always)]
fn is_leading_or_invalid_utf8_byte(b: u8) -> bool {
    // In the ASCII case, the most significant bit is never set. The leading
    // byte of a 2/3/4-byte sequence always has the top two most significant
    // bits set. For bytes that can never appear anywhere in valid UTF-8, this
    // also returns true, since every such byte has its two most significant
    // bits set:
    //
    //     \xC0 :: 11000000
    //     \xC1 :: 11000001
    //     \xF5 :: 11110101
    //     \xF6 :: 11110110
    //     \xF7 :: 11110111
    //     \xF8 :: 11111000
    //     \xF9 :: 11111001
    //     \xFA :: 11111010
    //     \xFB :: 11111011
    //     \xFC :: 11111100
    //     \xFD :: 11111101
    //     \xFE :: 11111110
    //     \xFF :: 11111111
    (b & 0b1100_0000) != 0b1000_0000
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn next() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"ab\xFF\xF0\x90\x8Dcd");
        // should contain two replacement chars for xff and the rest 3

        let mut chars = pt.chars();

        while let Some((start, end, ch)) = chars.next() {
            println!("{start}..{end} {ch}");
        }
        // assert_eq!(Some((0, 'a')), chars.next());
        // assert_eq!(Some((1, 'b')), chars.next());
        // assert_eq!(Some((2, REPLACEMENT_CHAR)), chars.next());
        // assert_eq!(Some((7, 'b')), chars.next());
        // assert_eq!(Some((8, 'a')), chars.next());
        // assert_eq!(None, chars.next());
    }

    // #[test]
    // fn prev() {
    //     let mut pt = PieceTree::new();
    //     pt.insert(0, b"ab\xFF\xFF\xFF\xFF\xFFba");
    //     let mut chars = pt.chars_at(pt.len());

    //     assert_eq!(Some((8, 'a')), chars.prev());
    //     assert_eq!(Some((7, 'b')), chars.prev());
    //     assert_eq!(Some((6, REPLACEMENT_CHAR)), chars.prev());
    //     assert_eq!(Some((1, 'b')), chars.prev());
    //     assert_eq!(Some((0, 'a')), chars.prev());
    //     assert_eq!(None, chars.prev());
    // }

    // #[test]
    // fn next_then_prev() {
    //     let mut pt = PieceTree::new();
    //     pt.insert(0, b"ab\xFF\xFF\xFF\xFF\xFFba");

    //     let mut chars = pt.chars();
    //     assert_eq!(Some((0, 'a')), chars.next());
    //     assert_eq!(Some((1, 'b')), chars.next());
    //     assert_eq!(Some((2, REPLACEMENT_CHAR)), chars.next());
    //     assert_eq!(Some((7, 'b')), chars.next());
    //     assert_eq!(Some((7, 'b')), chars.prev());
    //     assert_eq!(Some((6, REPLACEMENT_CHAR)), chars.prev());
    //     assert_eq!(Some((1, 'b')), chars.prev());
    //     assert_eq!(Some((0, 'a')), chars.prev());
    //     assert_eq!(None, chars.prev());
    // }

    // #[test]
    // fn middle_of_char() {
    //     let mut pt = PieceTree::new();
    //     pt.insert_str(0, "§ab");

    //     let slice = pt.slice(1..);
    //     let mut chars = slice.chars();
    //     assert_eq!(Some((0, REPLACEMENT_CHAR)), chars.next());
    //     assert_eq!(Some((1, 'a')), chars.next());
    //     assert_eq!(Some((2, 'b')), chars.next());
    //     assert_eq!(None, chars.next());
    // }

    // #[test]
    // fn multi_byte() {
    //     let mut pt = PieceTree::new();
    //     const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
    //     pt.insert_str(0, CONTENT);
    //     let mut chars = pt.chars();

    //     assert_eq!(Some((0, '❤')), chars.next());
    //     assert_eq!(Some((3, '🤍')), chars.next());
    //     assert_eq!(Some((7, '🥳')), chars.next());
    //     assert_eq!(Some((11, '❤')), chars.next());
    //     assert_eq!(Some((14, '\u{fe0f}')), chars.next());
    //     assert_eq!(Some((17, '간')), chars.next());
    //     assert_eq!(Some((20, '÷')), chars.next());
    //     assert_eq!(Some((22, '나')), chars.next());
    //     assert_eq!(Some((25, '는')), chars.next());
    //     assert_eq!(Some((28, '산')), chars.next());
    //     assert_eq!(Some((31, '다')), chars.next());
    //     assert_eq!(Some((34, '⛄')), chars.next());
    //     assert_eq!(None, chars.next());
    // }

    // #[test]
    // fn multi_byte_slice() {
    //     let mut pt = PieceTree::new();
    //     const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
    //     pt.insert_str(0, CONTENT);
    //     let slice = pt.slice(5..20);
    //     let mut chars = slice.chars();

    //     assert_eq!(Some((0, REPLACEMENT_CHAR)), chars.next());
    //     assert_eq!(Some((2, '🥳')), chars.next());
    //     assert_eq!(Some((6, '❤')), chars.next());
    //     assert_eq!(Some((9, '\u{fe0f}')), chars.next());
    //     assert_eq!(Some((12, '간')), chars.next());
    //     assert_eq!(None, chars.next());
    // }

    // #[test]
    // fn multi_byte_slice_prev() {
    //     let mut pt = PieceTree::new();
    //     const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
    //     pt.insert_str(0, CONTENT);
    //     let slice = pt.slice(5..20);
    //     let mut chars = slice.chars_at(slice.len());

    //     assert_eq!(Some((12, '간')), chars.prev());
    //     assert_eq!(Some((9, '\u{fe0f}')), chars.prev());
    //     assert_eq!(Some((6, '❤')), chars.prev());
    //     assert_eq!(Some((2, '🥳')), chars.prev());
    //     assert_eq!(Some((0, REPLACEMENT_CHAR)), chars.prev());
    //     assert_eq!(None, chars.prev());
    // }
}
