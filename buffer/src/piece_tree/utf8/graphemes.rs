use crate::piece_tree::slice::PieceTreeSlice;

// TODO take in a slice instead
pub fn next_grapheme_boundary(pt: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = pt.chars_at(pos);
    let mut ch = chars.next();

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
            None => return pt.len(),
        }

        let chunk = unsafe { std::str::from_utf8_unchecked(&buf[..valid_to]) };

        use unicode_segmentation::GraphemeIncomplete::*;
        match gc.next_boundary(chunk, ch_pos) {
            Ok(Some(bound)) => return bound,
            Ok(None) => return pt.len(),
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

        while start != pt.len() {
            let end = next_grapheme_boundary(&pt, start);
            println!(
                "Next grapheme boundary multibyte: {}, {}",
                end,
                &CONTENT[start..end]
            );
            start = end;
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
