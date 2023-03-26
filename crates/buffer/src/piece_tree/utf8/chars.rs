use std::ops::Range;

use crate::piece_tree::{Bytes, PieceTree};

const REPLACEMENT_CHAR: char = '\u{FFFD}';
const ACCEPT: u32 = 0;
const REJECT: u32 = 12;

// The first part of the table maps bytes to character classes that
// to reduce the size of the transition table and create bitmasks.
#[rustfmt::skip]
const CHAR_CLASSES: [u8; 256] = [
     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
     0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
     1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,  9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
     7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,  7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
     8,8,2,2,2,2,2,2,2,2,2,2,2,2,2,2,  2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
    10,3,3,3,3,3,3,3,3,3,3,3,3,4,3,3, 11,6,6,6,5,8,8,8,8,8,8,8,8,8,8,8,
];

// The second part is a transition table that maps a combination
// of a state of the automaton and a character class to a state.
#[rustfmt::skip]
const TRANSITIONS: [u8; 108] = [
    0, 12,24,36,60,96,84,12,12,12,48,72, 12,12,12,12,12,12,12,12,12,12,12,12,
    12, 0,12,12,12,12,12, 0,12, 0,12,12, 12,24,12,12,12,12,12,24,12,24,12,12,
    12,12,12,12,12,12,12,24,12,12,12,12, 12,24,12,12,12,12,12,12,12,24,12,12,
    12,12,12,12,12,12,12,36,12,36,12,12, 12,36,12,12,12,12,12,36,12,36,12,12,
    12,36,12,12,12,12,12,12,12,12,12,12,
];

const TRANSITIONS_BACKWARDS: [u8; 84] = [
    0, 24, 12, 12, 12, 12, 12, 24, 12, 24, 12, 12, 0, 24, 12, 12, 12, 12, 12, 24, 12, 24, 12,
    12, 12, 36, 0, 12, 12, 12, 12, 48, 12, 36, 12, 12, 12, 60, 12, 0, 0, 12, 12, 72, 12, 72,
    12, 12, 12, 60, 12, 0, 12, 12, 12, 72, 12, 72, 0, 12, 12, 12, 12, 12, 12, 0, 0, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 0,
];

// https://bjoern.hoehrmann.de/utf-8/decoder/dfa/
fn decode(state: &mut u32, cp: &mut u32, byte: u8) -> u32 {
    let byte = byte as u32;
    let class = CHAR_CLASSES[byte as usize];
    if *state != ACCEPT {
        *cp = (byte & 0x3f) | (*cp << 6);
    } else {
        *cp = (0xff >> class) & byte;
    }
    *state = TRANSITIONS[(*state + (class as u32)) as usize] as u32;
    *state
}

pub enum DecodeResult {
    Char(char),
    Invalid,
    Incomplete,
}

#[derive(Debug, Clone)]
struct Decoder {
    state: u32,
    cp: u32,
}

impl Decoder {
    pub fn new() -> Decoder {
        Decoder {
            state: ACCEPT,
            cp: 0,
        }
    }

    pub fn next(&mut self, byte: u8) -> DecodeResult {
        use DecodeResult::*;

        // ~12% better performance for ascii
        if self.state == ACCEPT && byte.is_ascii() {
            let ch = unsafe { char::from_u32_unchecked(byte as u32) };
            return Char(ch);
        }

        match decode(&mut self.state, &mut self.cp, byte) {
            ACCEPT => {
                // Automaton ensures this is safe
                let ch = unsafe { char::from_u32_unchecked(self.cp) };
                Char(ch)
            }
            REJECT => Invalid,
            _ => Incomplete,
        }
    }
}

// https://gershnik.github.io/2021/03/24/reverse-utf8-decoding.html
fn decode_last(
    state: &mut u32,
    cp: &mut u32,
    shift: &mut u32,
    collect: &mut u32,
    byte: u8,
) -> u32 {
    let byte = byte as u32;
    let class = CHAR_CLASSES[byte as usize];
    *state = TRANSITIONS_BACKWARDS[(*state + class as u32) as usize] as u32;

    if *state <= REJECT {
        *collect |= ((0xff >> class) & byte) << *shift;
        *cp = *collect;
        *shift = 0;
        *collect = 0;
    } else {
        *collect |= (byte & 0x3f) << *shift;
        *shift += 6;
    }

    *state
}

#[derive(Debug, Clone)]
struct DecoderRev {
    state: u32,
    cp: u32,
    shift: u32,
    collect: u32,
}

impl DecoderRev {
    pub fn new() -> DecoderRev {
        DecoderRev {
            state: ACCEPT,
            cp: 0,
            shift: 0,
            collect: 0,
        }
    }

    pub fn prev(&mut self, byte: u8) -> DecodeResult {
        use DecodeResult::*;

        if self.state == ACCEPT && byte.is_ascii() {
            let ch = unsafe { char::from_u32_unchecked(byte as u32) };
            return Char(ch);
        }

        match decode_last(
            &mut self.state,
            &mut self.cp,
            &mut self.shift,
            &mut self.collect,
            byte,
        ) {
            ACCEPT => {
                // Automaton ensures this is safe
                let ch = unsafe { char::from_u32_unchecked(self.cp) };
                Char(ch)
            }
            REJECT => Invalid,
            _ => Incomplete,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chars<'a> {
    bytes: Bytes<'a>,
    decoder: Decoder,
    decoder_rev: DecoderRev,
    rev_invalid_count: usize,
}

impl<'a> Chars<'a> {
    #[inline]
    pub fn new(pt: &'a PieceTree, at: usize) -> Chars<'a> {
        let bytes = Bytes::new(pt, at);
        Chars {
            bytes,
            decoder: Decoder::new(),
            decoder_rev: DecoderRev::new(),
            rev_invalid_count: 0,
        }
    }

    #[inline]
    pub(crate) fn new_from_slice(
        pt: &'a PieceTree,
        at: usize,
        range: Range<usize>,
    ) -> Chars<'a> {
        debug_assert!(
            range.end - range.start >= at,
            "Attempting to index {} over slice len {} ",
            at,
            range.end - range.start,
        );
        let bytes = Bytes::new_from_slice(pt, at, range);
        Chars {
            bytes,
            decoder: Decoder::new(),
            decoder_rev: DecoderRev::new(),
            rev_invalid_count: 0,
        }
    }

    pub fn next(&mut self) -> Option<(usize, usize, char)> {
        use DecodeResult::*;
        let start = self.bytes.pos();
        loop {
            let byte = match self.bytes.next() {
                Some(b) => b,
                None => {
                    let end = self.bytes.pos();
                    if start != end {
                        return Some((start, end, REPLACEMENT_CHAR));
                    } else {
                        return None;
                    }
                }
            };

            match self.decoder.next(byte) {
                Char(ch) => {
                    let end = self.bytes.pos();
                    return Some((start, end, ch));
                }
                Invalid => {
                    let end = self.bytes.pos();
                    return Some((start, end, REPLACEMENT_CHAR));
                }
                Incomplete => {}
            }
        }
    }

    pub fn prev(&mut self) -> Option<(usize, usize, char)> {
        use DecodeResult::*;

        if self.rev_invalid_count != 0 {
            let end = self.bytes.pos() + self.rev_invalid_count;
            let start = end.saturating_sub(1);
            self.rev_invalid_count -= 1;
            return Some((start, end, REPLACEMENT_CHAR));
        }

        let end = self.bytes.pos();
        loop {
            let byte = match self.bytes.prev() {
                Some(b) => b,
                None => {
                    let start = self.bytes.pos();
                    if start != end {
                        return Some((start, end, REPLACEMENT_CHAR));
                    } else {
                        return None;
                    }
                }
            };

            match self.decoder_rev.prev(byte) {
                Char(ch) => {
                    println!("OK: {}", self.bytes.pos());
                    let start = self.bytes.pos();
                    return Some((start, end, ch));
                }
                Invalid => {
                    println!("INVALID: {}, {}", self.bytes.pos(), is_leading_or_invalid_utf8_byte(byte));
                    let start = self.bytes.pos();
                    self.rev_invalid_count = (end - start).saturating_sub(1);
                    return Some((end.saturating_sub(1), end, REPLACEMENT_CHAR));
                }
                Incomplete => {
                    println!("INCOMPL: {}", self.bytes.pos());
                }
            }
        }
    }
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
        pt.insert(0, b"ab\xFF\xF0\x90\x8D\xFF\x90\x8Dcd");

        let mut chars = pt.chars();

        assert_eq!(Some((0, 1, 'a')), chars.next());
        assert_eq!(Some((1, 2, 'b')), chars.next());
        assert_eq!(Some((2, 3, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((3, 6, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((6, 7, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((7, 8, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((8, 9, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((9, 10, 'c')), chars.next());
        assert_eq!(Some((10, 11, 'd')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"ab\xFF\xF0\x90\x8D\xFF\x90\x8Dcd");
        let mut chars = pt.chars_at(pt.len());

        assert_eq!(Some((10, 11, 'd')), chars.prev());
        assert_eq!(Some((9, 10, 'c')), chars.prev());
        assert_eq!(Some((8, 9, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((7, 8, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((6, 7, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((3, 6, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((2, 3, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((1, 2, 'b')), chars.prev());
        assert_eq!(Some((0, 1, 'a')), chars.prev());
        assert_eq!(None, chars.prev());
    }

    #[test]
    fn next_then_prev() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"ab\xFF\xFFba");

        let mut chars = pt.chars();
        assert_eq!(Some((0, 1, 'a')), chars.next());
        assert_eq!(Some((1, 2, 'b')), chars.next());
        assert_eq!(Some((2, 3, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((3, 4, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((4, 5, 'b')), chars.next());
        assert_eq!(Some((4, 5, 'b')), chars.prev());
        assert_eq!(Some((3, 4, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((2, 3, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((1, 2, 'b')), chars.prev());
        assert_eq!(Some((0, 1, 'a')), chars.prev());
        assert_eq!(None, chars.prev());
    }

    #[test]
    fn middle_of_char() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "§ab");

        let slice = pt.slice(1..);
        let mut chars = slice.chars();
        assert_eq!(Some((0, 1, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((1, 2, 'a')), chars.next());
        assert_eq!(Some((2, 3, 'b')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);
        let mut chars = pt.chars();

        assert_eq!(Some((0, 3, '❤')), chars.next());
        assert_eq!(Some((3, 7, '🤍')), chars.next());
        assert_eq!(Some((7, 11, '🥳')), chars.next());
        assert_eq!(Some((11, 14, '❤')), chars.next());
        assert_eq!(Some((14, 17, '\u{fe0f}')), chars.next());
        assert_eq!(Some((17, 20, '간')), chars.next());
        assert_eq!(Some((20, 22, '÷')), chars.next());
        assert_eq!(Some((22, 25, '나')), chars.next());
        assert_eq!(Some((25, 28, '는')), chars.next());
        assert_eq!(Some((28, 31, '산')), chars.next());
        assert_eq!(Some((31, 34, '다')), chars.next());
        assert_eq!(Some((34, 37, '⛄')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn multi_byte_slice() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);
        let slice = pt.slice(5..20);
        let mut chars = slice.chars();

        assert_eq!(Some((0, 1, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((1, 2, REPLACEMENT_CHAR)), chars.next());
        assert_eq!(Some((2, 6, '🥳')), chars.next());
        assert_eq!(Some((6, 9, '❤')), chars.next());
        assert_eq!(Some((9, 12, '\u{fe0f}')), chars.next());
        assert_eq!(Some((12, 15, '간')), chars.next());
        assert_eq!(None, chars.next());
    }

    #[test]
    fn multi_byte_slice_prev() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);
        let slice = pt.slice(5..20);
        let mut chars = slice.chars_at(slice.len());

        assert_eq!(Some((12, 15, '간')), chars.prev());
        assert_eq!(Some((9, 12, '\u{fe0f}')), chars.prev());
        assert_eq!(Some((6, 9, '❤')), chars.prev());
        assert_eq!(Some((2, 6, '🥳')), chars.prev());
        assert_eq!(Some((1, 2, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(Some((0, 1, REPLACEMENT_CHAR)), chars.prev());
        assert_eq!(None, chars.prev());
    }
}
