use unicode_segmentation::GraphemeCursor;

use crate::piece_tree::slice::PieceTreeSlice;

pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = slice.chars_at(pos);
    let mut ch = chars.next();

    let mut ch_pos = 0;
    let mut valid_to;
    let mut buf = [0u8; 4];

    let mut gc = GraphemeCursor::new(0, slice.len(), true);

    loop {
        match ch {
            Some((pos, ch)) => {
                ch.encode_utf8(&mut buf);
                valid_to = ch.len_utf8();
                ch_pos += ch.len_utf8();
            }
            None => return slice.len(),
        }

        let chunk = unsafe { std::str::from_utf8_unchecked(&buf[..valid_to]) };

        use unicode_segmentation::GraphemeIncomplete::*;
        match gc.next_boundary(chunk, ch_pos) {
            Ok(Some(bound)) => return bound,
            Ok(None) => return slice.len(),
            Err(e) => match e {
                PreContext(pos) => {
                    let mut pre_chars = slice.chars_at(pos);
                    let (pos, ch) = pre_chars
                        .prev()
                        .expect("Precontext: Cannot find char ending at {pos}");
                    let mut buf = [0u8; 4];
                    ch.encode_utf8(&mut buf);
                    let valid_to = ch.len_utf8();
                    let chunk = unsafe { std::str::from_utf8_unchecked(&buf[..valid_to]) };
                    gc.provide_context(chunk, pos);
                }
                NextChunk => {
                    ch = chars.next();
                }
                _ => unreachable!(),
            },
        }
    }
}

pub fn prev_grapheme_boundary(pt: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = pt.chars_at(pos);
    let mut ch = chars.prev();

    let mut ch_pos;
    let mut valid_to;
    let mut buf = [0u8; 4];

    let mut gc = unicode_segmentation::GraphemeCursor::new(pos, usize::MAX, true);

    loop {
        match ch {
            Some((pos, ch)) => {
                ch.encode_utf8(&mut buf);
                valid_to = ch.len_utf8();
                ch_pos = pos;
            }
            None => return 0,
        }

        let chunk = unsafe { std::str::from_utf8_unchecked(&buf[..valid_to]) };

        use unicode_segmentation::GraphemeIncomplete::*;
        match gc.prev_boundary(chunk, ch_pos) {
            Ok(Some(bound)) => return bound,
            Ok(None) => return 0,
            Err(e) => match e {
                PreContext(pos) => {
                    let mut pre_chars = pt.chars_at(pos);
                    let (pos, ch) = pre_chars
                        .prev()
                        .expect("Precontext: Cannot find char ending at {pos}");
                    let mut buf = [0u8; 4];
                    ch.encode_utf8(&mut buf);
                    let valid_to = ch.len_utf8();
                    let chunk = unsafe { std::str::from_utf8_unchecked(&buf[..valid_to]) };
                    gc.provide_context(chunk, pos);
                }
                PrevChunk => {
                    ch = chars.prev();
                }
                _ => unreachable!(),
            },
        }
    }
}

#[inline]
pub fn next_grapheme<'a>(slice: &'a PieceTreeSlice, pos: usize) -> Option<PieceTreeSlice<'a>> {
    let start = pos;
    let end = next_grapheme_boundary(slice, pos);
    if start == end {
        return None;
    }

    Some(slice.slice(start..end))
}

#[inline]
pub fn prev_grapheme<'a>(slice: &'a PieceTreeSlice, pos: usize) -> Option<PieceTreeSlice<'a>> {
    let start = pos;
    let end = prev_grapheme_boundary(slice, pos);
    if start == end {
        return None;
    }

    Some(slice.slice(start..end))
}

#[cfg(test)]
mod test {
    use crate::piece_tree::PieceTree;

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

        let boundaries = [3, 7, 11, 17, 20, 22, 25, 28, 31, 34];
        let mut pos = 0;
        let slice = pt.slice(..);

        for boundary in boundaries {
            pos = next_grapheme_boundary(&slice, pos);
            assert_eq!(boundary, pos);
        }
    }

    #[test]
    fn next_grapheme_boundary_multi_byte_slice() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);

        let boundaries = [3, 7, 11, 17, 20, 22, 25, 28, 31, 34];
        let mut pos = 0;
        let slice = pt.slice(5..20);

        for boundary in boundaries {
            pos = next_grapheme_boundary(&slice, pos);
            assert_eq!(boundary, pos);
        }
    }

    // #[test]
    // fn prev_grapheme_boundary_multi_byte() {
    //     let mut pt = PieceTree::new();
    //     const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
    //     pt.insert_str(0, CONTENT);

    //     let mut start = CONTENT.len();

    //     println!("content: {:?}", CONTENT.as_bytes());
    //     while let Some(end) = prev_grapheme_boundary(&pt, start) {
    //         println!(
    //             "Prev grapheme boundary multibyte: {end}..{start} {}",
    //             &CONTENT[end..start],
    //         );
    //         start = end;
    //     }
    // }

    // #[test]
    // fn next_grapheme_test() {
    //     const CONTENT: &[u8] = "❤️".as_bytes();
    //     println!("TEST {:?}", CONTENT.grapheme_indices().next());
    // }
}
