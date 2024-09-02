use crate::{
    piece_tree::{Bytes, PieceTreeView},
    PieceTreeSlice,
};

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
    0, 24, 12, 12, 12, 12, 12, 24, 12, 24, 12, 12, 0, 24, 12, 12, 12, 12, 12, 24, 12, 24, 12, 12,
    12, 36, 0, 12, 12, 12, 12, 48, 12, 36, 12, 12, 12, 60, 12, 0, 0, 12, 12, 72, 12, 72, 12, 12,
    12, 60, 12, 0, 12, 12, 12, 72, 12, 72, 0, 12, 12, 12, 12, 12, 12, 0, 0, 12, 12, 12, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 0,
];

/// utf8 decode next scalar value
pub fn decode_utf8(bytes: &[u8]) -> (Option<char>, usize) {
    if bytes.is_empty() {
        return (None, 0);
    }
    let mut decoder = Decoder::new();
    let mut pos = 0;
    loop {
        use DecodeResult::*;
        match decoder.next(bytes[pos]) {
            Char(ch) => return (Some(ch), pos + 1),
            Invalid => return (None, pos + 1),
            Incomplete => {
                pos += 1;
                if pos >= bytes.len() {
                    return (None, bytes.len());
                }
            }
        }
    }
}

pub fn decode_utf8_iter(mut bytes: impl Iterator<Item = u8>) -> (Option<char>, u64) {
    let mut decoder = Decoder::new();
    let mut size = 0;
    while let Some(b) = bytes.next() {
        size += 1;

        use DecodeResult::*;
        match decoder.next(b) {
            Char(ch) => return (Some(ch), size),
            Invalid => return (None, size),
            Incomplete => {}
        }
    }

    (None, size)
}

// https://bjoern.hoehrmann.de/utf-8/decoder/dfa/
#[inline]
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
            REJECT => {
                self.reset();
                Invalid
            }
            _ => Incomplete,
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.state = ACCEPT;
        self.cp = 0;
    }
}

// https://gershnik.github.io/2021/03/24/reverse-utf8-decoding.html
#[inline]
fn decode_rev(state: &mut u32, cp: &mut u32, shift: &mut u32, collect: &mut u32, byte: u8) -> u32 {
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

        match decode_rev(
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
            REJECT => {
                self.reset();
                Invalid
            }
            _ => Incomplete,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.state = ACCEPT;
        self.cp = 0;
        self.shift = 0;
        self.collect = 0;
    }
}

#[derive(Debug, Clone)]
pub struct Chars<'a> {
    bytes: Bytes<'a>,
    decoder: Decoder,
    decoder_rev: DecoderRev,
}

impl<'a> Chars<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTreeView, at: u64) -> Chars<'a> {
        let bytes = Bytes::new(pt, at);
        Chars {
            bytes,
            decoder: Decoder::new(),
            decoder_rev: DecoderRev::new(),
        }
    }

    #[inline]
    pub(crate) fn new_from_slice(slice: &PieceTreeSlice<'a>, at: u64) -> Chars<'a> {
        debug_assert!(
            slice.len() >= at,
            "Attempting to index {} over slice len {} ",
            at,
            slice.len(),
        );
        let bytes = Bytes::new_from_slice(slice, at);
        Chars {
            bytes,
            decoder: Decoder::new(),
            decoder_rev: DecoderRev::new(),
        }
    }

    pub fn next(&mut self) -> Option<(u64, u64, char)> {
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
                    // We may have a valid prefix of utf8.
                    // Replace it with one replacement character.
                    let mut end = self.bytes.pos();
                    let read_len = end - start;
                    if read_len > 1 {
                        self.bytes.prev();
                        end -= 1;
                    }
                    return Some((start, end, REPLACEMENT_CHAR));
                }
                Incomplete => {}
            }
        }
    }

    pub fn prev(&mut self) -> Option<(u64, u64, char)> {
        use DecodeResult::*;

        let end = self.bytes.pos();
        loop {
            let byte = match self.bytes.prev() {
                Some(b) => b,
                None => {
                    let mut start = self.bytes.pos();
                    let read_len = end - start;
                    // We for sure have an invalid utf8 prefix
                    for _ in 1..read_len {
                        self.bytes.next();
                        start += 1;
                    }
                    if read_len > 0 {
                        return Some((start, end, REPLACEMENT_CHAR));
                    } else {
                        return None;
                    }
                }
            };

            match self.decoder_rev.prev(byte) {
                Char(ch) => {
                    let start = self.bytes.pos();
                    return Some((start, end, ch));
                }
                Invalid => {
                    // We have a valid suffix of a utf8 sequence.
                    // But not a valid codepoint.
                    //
                    // To handle errors like the forward automaton, determine
                    // utf8 sequence length from the first byte.
                    // If the length is in range [0,4] and we have read less than length
                    // we have a valid utf8 prefix and replace the whole
                    // prefix with one replacement character.
                    //
                    // Otherwise replace every byte with its own replacement
                    // character, as a valid suffix contains only continuation
                    // bytes start with 10 they definitely are not valid lead
                    // bytes.
                    let mut start = self.bytes.pos();
                    let seq_len = byte.count_ones() as u64;
                    let read_len = end - start;
                    let valid_prefix = read_len < seq_len && seq_len >= 2 && seq_len <= 4;

                    if !valid_prefix {
                        for _ in 1..read_len {
                            self.bytes.next();
                            start += 1;
                        }
                    }

                    return Some((start, end, REPLACEMENT_CHAR));
                }
                Incomplete => {}
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::PieceTree;

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
        pt.insert(0, "§ab");

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
        pt.insert(0, CONTENT);
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
        pt.insert(0, CONTENT);
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
        pt.insert(0, CONTENT);
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
