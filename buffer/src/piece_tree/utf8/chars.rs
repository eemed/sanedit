use std::ops::Range;

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
        const LEN: usize = 4;
        let mut buf = [0u8; LEN];
        let mut read = 0;

        loop {
            match decode_char(&buf[..read]) {
                DecodeResult::Invalid(valid_up_to) => {
                    // TODO is there better way than to scroll back.
                    // Maybe store this in a buf, store last direction bool, and
                    // scroll back by buffer if needed?
                    // Not a high priority because we are handling invalid utf8
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

    pub fn prev(&mut self) -> Option<(usize, usize, char)> {
        let end = self.bytes.pos();
        const LEN: usize = 4;
        let mut buf = [0u8; LEN];
        let mut read = 0;

        // Read to atleast one byte stop if is leading byte, or invalid byte, or 4 bytes total
        // read. Fill buf backwards
        while let Some(byte) = self.bytes.prev() {
            buf[LEN - 1 - read] = byte;
            read += 1;
            if read == LEN || is_leading_or_invalid_utf8_byte(byte) {
                break;
            }
        }

        if read == 0 {
            return None;
        }

        let start = end - read;

        match decode_char(&buf[LEN - read..]) {
            DecodeResult::Invalid(_) => {
                // We have an invalid byte, restore the iter
                let extra_bytes_read = read - 1;
                for _ in 0..extra_bytes_read {
                    self.bytes.next();
                }
                Some((start + extra_bytes_read, end, REPLACEMENT_CHAR))
            }
            DecodeResult::Ok(ch) => Some((start, end, ch)),
            DecodeResult::Incomplete => Some((start, end, REPLACEMENT_CHAR)),
        }
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

pub(crate) mod chars2 {
    use super::REPLACEMENT_CHAR;
    use std::ops::Range;
    use crate::piece_tree::{Bytes, PieceTree};

    const ACCEPT: u32 = 0;
    const REJECT: u32 = 1;

    #[rustfmt::skip]
    const CHAR_CLASSES: [u8; 256] = [
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 00..1f
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 20..3f
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 40..5f
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 60..7f
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9, // 80..9f
      7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7, // a0..bf
      8,8,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, // c0..df
      0xa,0x3,0x3,0x3,0x3,0x3,0x3,0x3,0x3,0x3,0x3,0x3,0x3,0x4,0x3,0x3, // e0..ef
      0xb,0x6,0x6,0x6,0x5,0x8,0x8,0x8,0x8,0x8,0x8,0x8,0x8,0x8,0x8,0x8, // f0..ff
    ];

    #[rustfmt::skip]
    const TRANSITIONS: [u8; 144] = [
      0x0,0x1,0x2,0x3,0x5,0x8,0x7,0x1,0x1,0x1,0x4,0x6,0x1,0x1,0x1,0x1, // s0..s0
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,1,1,1,1,1,0,1,0,1,1,1,1,1,1, // s1..s2
      1,2,1,1,1,1,1,2,1,2,1,1,1,1,1,1,1,1,1,1,1,1,1,2,1,1,1,1,1,1,1,1, // s3..s4
      1,2,1,1,1,1,1,1,1,2,1,1,1,1,1,1,1,1,1,1,1,1,1,3,1,3,1,1,1,1,1,1, // s5..s6
      1,3,1,1,1,1,1,3,1,3,1,1,1,1,1,1,1,3,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // s7..s8
    ];

    fn decode(state: &mut u32, cp: &mut u32, byte: u32) -> u32 {
        let t = CHAR_CLASSES[byte as usize];
        if *state != ACCEPT {
            *cp = (byte & 0x3f) | (*cp << 6);
        } else {
            *cp = (0xff >> t) & byte;
        }
        *state = TRANSITIONS[(*state * 16 + (t as u32)) as usize] as u32;
        *state
    }

    // https://bjoern.hoehrmann.de/utf-8/decoder/dfa/
    #[derive(Debug, Clone)]
    struct Decoder {
        state: u32,
        cp: u32,
    }

    impl Decoder {
        pub fn new() -> Decoder {
            Decoder {
                state: 0,
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

            match decode(&mut self.state, &mut self.cp, byte as u32) {
                ACCEPT => {
                    // Automaton ensures this is safe
                    let ch = unsafe { char::from_u32_unchecked(self.cp) };
                    Char(ch)
                },
                REJECT => Invalid,
                _ => Incomplete,
            }
        }
    }


    pub enum DecodeResult {
        Char(char),
        Invalid,
        Incomplete,
    }

    #[derive(Debug, Clone)]
    pub struct Chars2<'a> {
        bytes: Bytes<'a>,
        decoder: Decoder,
    }

    impl<'a> Chars2<'a> {
        #[inline]
        pub fn new(pt: &'a PieceTree, at: usize) -> Chars2<'a> {
            let bytes = Bytes::new(pt, at);
            Chars2 { bytes, decoder: Decoder::new() }
        }

        #[inline]
        pub(crate) fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Chars2<'a> {
            debug_assert!(
                range.end - range.start >= at,
                "Attempting to index {} over slice len {} ",
                at,
                range.end - range.start,
            );
            let bytes = Bytes::new_from_slice(pt, at, range);
            Chars2 { bytes, decoder: Decoder::new() }
        }

        pub fn next(&mut self) -> Option<(usize, usize, char)> {
            use DecodeResult::*;
            let start = self.bytes.pos();
            loop {
                let byte = self.bytes.next()?;

                match self.decoder.next(byte) {
                    Char(ch) => {
                        let end = self.bytes.pos();
                        return Some((start, end, ch))
                    }
                    Invalid => {
                        let end = self.bytes.pos();
                        return Some((start, end, REPLACEMENT_CHAR));
                    }
                    Incomplete => {},
                }
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        #[test]
        fn decode_test() {
            use DecodeResult::*;

            let mut decoder = Decoder::new();
            let bytes = b"hello \xc2\xA7b";
            println!("-- RESULT --");
            for b in bytes {
                match decoder.next(*b) {
                    Char(ch) => {
                        print!("{ch}")
                    }
                    Invalid => {
                        print!("\u{FFFD}");
                    }
                    Incomplete => {},
                }
            }
            println!("");
            println!("-- -- --");
        }
    }
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
