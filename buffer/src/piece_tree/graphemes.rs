use crate::cursor_iterator::CursorIterator;

use super::{Bytes, PieceTree};
use bstr::ByteSlice;

#[inline]
fn is_utf8_start(byte: u8) -> bool {
    (byte & 0xc0) != 0x80
}

// pub fn is_grapheme_boundary(pt: &PieceTree, pos: usize) -> bool {
//     let chunks = pt.chunks_at(pos);
//     let chunk = chunks.get();
//     let chunk_pos = chunks.pos();

//     if let Some(chk) = chunk {
//         let bytes = chk.as_ref();
//         let relative_pos = pos - chunk_pos;
//         bytes[relative_pos]
//     } else {
//         false
//     }
// }

pub fn next_grapheme_boundary(pt: &PieceTree, pos: usize) -> Option<usize> {
    let mut bytes = Bytes::new(pt, pos);
    let mut buf = Vec::new();

    loop {
        if let Some((_start, end, _grapheme)) = buf.grapheme_indices().next() {
            return Some(pos + end);
        } else {
            buf.push(bytes.get()?);

            let mut byte = bytes.next()?;
            while !is_utf8_start(byte) {
                buf.push(byte);
                byte = bytes.next()?;
            }
        }
    }
}

pub fn prev_grapheme_boundary(pt: &PieceTree, pos: usize) -> Option<usize> {
    todo!()
}

pub fn next_grapheme(pt: &PieceTree, pos: usize) -> Option<(usize, usize, String)> {
    todo!()
}

pub fn prev_grapheme(pt: &PieceTree, pos: usize) -> Option<(usize, usize, String)> {
    todo!()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn next_grapheme() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foobar");

        println!(
            "Next grapheme boundary: {:?}",
            next_grapheme_boundary(&pt, 0)
        );
    }

    #[test]
    fn next_grapheme_multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️";
        pt.insert_str(0, CONTENT);

        let mut start = 0;

        while let Some(end) = next_grapheme_boundary(&pt, start) {
            println!(
                "Next grapheme boundary multibyte: {:?}",
                &CONTENT[start..end]
            );
            start = end;
        }
    }
}
