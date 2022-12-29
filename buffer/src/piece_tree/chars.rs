use std::{cmp, ops::Range};

use bstr::ByteSlice;

use super::{
    chunks::{Chunk, Chunks},
    Bytes, PieceTree,
};

const REPLACEMENT: char = '\u{FFFD}';

#[derive(Debug, Clone)]
pub struct Chars<'a> {
    pt: &'a PieceTree,
    chunks: Chunks<'a>,
}

impl<'a> Chars<'a> {
    #[inline]
    pub fn new(pt: &'a PieceTree, at: usize) -> Chars<'a> {
        let chunks = Chunks::new(pt, at);
        Chars { pt, chunks }
    }

    #[inline]
    pub fn new_from_slice(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Chars<'a> {
        let chunks = Chunks::new_from_slice(pt, at, range);
        Chars { pt, chunks }
    }

    pub fn next(&mut self) -> (usize, char) {
        let pos_chunk = self.chunks.get();
        // bstr::decode_utf8()
        todo!()
    }

    pub fn prev(&mut self) -> (usize, char) {
        // bstr::decode_last_utf8();
        todo!()
    }
}

fn decode_last(bytes: &[u8]) {
    let (ch, size) = bstr::decode_last_utf8(&bytes);
    if let Some(ch) = ch {
        *bytes = &bytes[..bytes.len() - size];
        DecodeResult::Ok(ch)
    } else {
        let len = cmp::min(3, bytes.len());
        let partial = &bytes[bytes.len() - len..];
        // TODO
        let is_invalid = utf8_valid_up_to(partial) == 0;

        if is_invalid {
            DecodeResult::Invalid
        } else {
            DecodeResult::Incomplete
        }
    }
}

// Decodes a char from bytes and consumes the bytes decoded.
// If the result is Invalid or Incomplete no bytes are consumed
// TODO consume max amount of invalid byttes? does bstr decode_utf8 actually do
// this?
fn decode(mut bytes: &mut &[u8]) -> DecodeResult {
    let (ch, size) = bstr::decode_utf8(&bytes);
    if let Some(ch) = ch {
        *bytes = &bytes[size..];
        DecodeResult::Ok(ch)
    } else {
        let len = cmp::min(3, bytes.len());
        let partial = &bytes[..len];
        let is_invalid = utf8_valid_up_to(partial) == 0;

        if is_invalid {
            DecodeResult::Invalid
        } else {
            DecodeResult::Incomplete
        }
    }
}

enum DecodeResult {
    Invalid,
    Incomplete,
    Ok(char),
}

fn utf8_valid_up_to(bytes: &[u8]) -> usize {
    debug_assert!(bytes.len() < 4);

    match bytes.to_str() {
        Ok(n) => n.len(),
        Err(e) => e.valid_up_to(),
    }
}
