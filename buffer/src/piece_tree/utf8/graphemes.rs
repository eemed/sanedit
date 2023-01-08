use crate::piece_tree::slice::PieceTreeSlice;

pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut at_start = pos == 0;
    let mut chars = slice.chars_at(pos);
    let mut pre = None;
    let mut all: Vec<(usize, char)> = Vec::with_capacity(4);
    let mut buf = String::with_capacity(4);

    if let Some((pos, ch)) = chars.next() {
        all.push((pos, ch));
        buf.push(ch);
    }

    while let Some((pos, ch)) = chars.next() {
        all.push((pos, ch));
        buf.push(ch);

        use BoundaryResult::*;
        match find_next_boundary(&buf, at_start) {
            AtEnd => return slice.end(),
            Boundary(bound) => {
                // Find slice boundary from all graphemes, by counting the utf8
                // length of yielded chars and then returning their slice
                // position.
                let mut utf8_len = 0;
                for (pos, ch) in all.into_iter() {
                    if bound == utf8_len {
                        return pos;
                    }
                    utf8_len += ch.len_utf8();
                }

                return slice.end();
            }
            NeedPre => {
                pre = Some(pre.unwrap_or(slice.chars_at(pos)));
                // Safe to unwrap as we just created pre iter if it did not exist
                if let Some((pos, ch)) = pre.as_mut().unwrap().prev() {
                    all.insert(0, (pos, ch));
                    buf.insert(0, ch);
                } else {
                    at_start = true;
                }
            }
            NeedMore => {
                // automatically handled
            }
        }
    }

    return slice.end();
}

enum BoundaryResult {
    /// No boundary because we are at the end
    AtEnd,
    /// Found a boundary
    Boundary(usize),
    /// Need more data
    NeedMore,
    /// Need previous data
    NeedPre,
}

#[inline]
fn find_next_boundary(chunk: &str, at_start: bool) -> BoundaryResult {
    let gc_start = if at_start { 0 } else { 4 };
    let mut gc = unicode_segmentation::GraphemeCursor::new(gc_start, usize::MAX, true);

    use unicode_segmentation::GraphemeIncomplete::*;
    match gc.next_boundary(chunk, gc_start) {
        Ok(Some(bound)) => return BoundaryResult::Boundary(bound - gc_start),
        Ok(None) => return BoundaryResult::AtEnd,
        Err(e) => match e {
            PreContext(_pos) => return BoundaryResult::NeedPre,
            NextChunk => return BoundaryResult::NeedMore,
            _ => unreachable!(),
        },
    }
}

pub fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = slice.chars_at(pos);
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

        let boundaries = [7, 11, 17, 20];
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
