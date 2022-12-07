use crate::cursor_iterator::CursorIterator;

use super::{slice::PieceTreeSlice, Bytes, PieceTree};
use bstr::{BString, ByteSlice, B};

// Using bstr to convert bytes to grapheme clusters.
// bstr is not meant to be used on streaming

#[inline]
fn is_utf8_start(byte: u8) -> bool {
    (byte & 0xc0) != 0x80
}

// pub fn is_grapheme_boundary(pt: &PieceTree, pos: usize) -> bool {
// }

fn next_grapheme_impl<'a>(pt: &'a PieceTree, pos: usize) -> Option<PieceTreeSlice<'a>> {
    let chunks = pt.chunks_at(pos);
    let chunk = chunks.get()?;
    let chunk_pos = chunks.pos();

    let is_last = chunk_pos + chunk.as_ref().len() == pt.len();
    let relative_pos = pos - chunk_pos;
    let bytes = &chunk.as_ref()[relative_pos..];
    if let Some((start, end, _grapheme)) = bytes.grapheme_indices().next() {
        if is_last || end != bytes.len() {
            let slice = pt.slice(pos + start..pos + end);
            return Some(slice);
        }
    }

    next_grapheme_impl_copy(pt, pos)
}

#[inline]
fn read_next_codepoint(bytes: &mut Bytes, buf: &mut BString) -> bool {
    match bytes.get() {
        Some(b) => buf.push(b),
        None => return false,
    }

    while let Some(b) = bytes.next() {
        if is_utf8_start(b) {
            break;
        }

        buf.push(b);
    }

    true
}

#[inline]
fn next_grapheme_impl_copy<'a>(pt: &'a PieceTree, pos: usize) -> Option<PieceTreeSlice<'a>> {
    let mut bytes = Bytes::new(pt, pos);
    let mut buf = BString::default();

    loop {
        let at_end = !read_next_codepoint(&mut bytes, &mut buf);

        if let Some((start, end, _grapheme)) = buf.grapheme_indices().next() {
            if at_end || end != buf.len() {
                let slice = pt.slice(pos + start..pos + end);
                return Some(slice);
            }
        } else if at_end {
            return None;
        }
    }
}

#[inline]
fn read_prev_codepoint(bytes: &mut Bytes, buf: &mut Vec<u8>) -> bool {
    let len = buf.len();
    while let Some(b) = bytes.prev() {
        buf.insert(0, b);

        if is_utf8_start(b) {
            break;
        }
    }

    len != buf.len()
}

fn prev_grapheme_impl_copy<'a>(pt: &'a PieceTree, pos: usize) -> Option<PieceTreeSlice<'a>> {
    let mut bytes = Bytes::new(pt, pos);
    let mut buf = Vec::new();
    let mut prev_match_len = None;

    loop {
        if !read_prev_codepoint(&mut bytes, &mut buf) {
            todo!()
            // return prev_match_len.map(|p| pos - p);
        }

        if let Some((start, end, grapheme)) = buf.grapheme_indices().next_back() {
            let end_matches_prev = prev_match_len.map_or(false, |prev| prev == end - start);
            if end_matches_prev {
                todo!()
                // return Some(pos - (end - start));
            }

            prev_match_len = Some(end - start);
        }
    }
}

fn prev_grapheme_impl<'a>(pt: &'a PieceTree, pos: usize) -> Option<PieceTreeSlice<'a>> {
    todo!()
}

#[inline]
pub fn next_grapheme_boundary(pt: &PieceTree, pos: usize) -> Option<usize> {
    let slice = next_grapheme_impl(pt, pos)?;
    Some(slice.end())
}

#[inline]
pub fn prev_grapheme_boundary(pt: &PieceTree, pos: usize) -> Option<usize> {
    let slice = prev_grapheme_impl(pt, pos)?;
    Some(slice.start())
}

#[inline]
pub fn next_grapheme<'a>(pt: &'a PieceTree, pos: usize) -> Option<PieceTreeSlice<'a>> {
    next_grapheme_impl(pt, pos)
}

#[inline]
pub fn prev_grapheme(pt: &PieceTree, pos: usize) -> Option<PieceTreeSlice<'a>> {
    prev_grapheme_impl(pt, pos)
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn next_grapheme() {
    //     let mut pt = PieceTree::new();
    //     pt.insert_str(0, "foobar");

    //     println!(
    //         "Next grapheme boundary: {:?}",
    //         next_grapheme_boundary(&pt, 0)
    //     );
    // }

    // #[test]
    // fn next_grapheme_boundary_multi_byte() {
    // }

    #[test]
    fn next_grapheme_boundary_multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);

        let mut start = 0;

        while let Some(end) = next_grapheme_boundary(&pt, start) {
            println!(
                "Next grapheme boundary multibyte: {}, {}",
                end,
                &CONTENT[start..end]
            );
            start = end;
        }
    }

    #[test]
    fn prev_grapheme_boundary_multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);

        let mut start = CONTENT.len();

        println!("content: {:?}", CONTENT.as_bytes());
        while let Some(end) = prev_grapheme_boundary(&pt, start) {
            println!(
                "Prev grapheme boundary multibyte: {end}..{start} {}",
                &CONTENT[end..start],
            );
            start = end;
        }
    }

    // #[test]
    // fn next_grapheme_test() {
    //     const CONTENT: &[u8] = "❤️".as_bytes();
    //     println!("TEST {:?}", CONTENT.grapheme_indices().next());
    // }
}
