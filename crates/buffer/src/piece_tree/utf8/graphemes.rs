use sanedit_ucd::{GraphemeBreak, Property};

use crate::piece_tree::PieceTreeSlice;

use super::chars::Chars;

#[inline]
pub fn prev_grapheme<'a>(slice: &'a PieceTreeSlice, pos: usize) -> Option<PieceTreeSlice<'a>> {
    let end = pos;
    let start = prev_grapheme_boundary(slice, pos);
    if start == end {
        return None;
    }

    Some(slice.slice(start..end))
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

pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = slice.chars_at(pos);
    let mut current = chars.next();
    let mut after = chars.next();

    loop {
        match (current, after) {
            (Some((first, _, a)), Some((second, _, b))) => {
                if is_break(slice, first, a, b) {
                    return second;
                }

                // Progress forward
                current = after;
                after = chars.next();
            }
            (None, None) => return slice.len(),
            (Some(_), None) => return slice.len(),
            (None, Some(_)) => unreachable!(),
        }
    }
}

pub fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = slice.chars_at(pos);
    let mut after = chars.prev();
    let mut current = chars.prev();

    loop {
        match (current, after) {
            (Some((first, _, a)), Some((second, _, b))) => {
                if is_break(slice, first, a, b) {
                    return second;
                }

                // Progress backwards
                after = current;
                current = chars.prev();
            }
            (None, None) => return 0,
            (None, Some(_)) => return 0,
            (Some(_), None) => unreachable!(),
        }
    }
}

fn is_break(slice: &PieceTreeSlice, start: usize, a: char, b: char) -> bool {
    use BreakResult::*;

    match pair_break(a, b) {
        Break => true,
        Emoji => {
            let mut before = slice.chars_at(start);
            is_break_emoji(&mut before)
        }
        Regional => {
            let mut before = slice.chars_at(start);
            is_break_regional(&mut before)
        }
        NoBreak => false,
    }
}

// https://www.unicode.org/reports/tr29/
/// Check if a grapheme break exists between the two characters
fn pair_break(before: char, after: char) -> BreakResult {
    use sanedit_ucd::GraphemeBreak::*;
    use BreakResult::*;

    let before_gb = sanedit_ucd::grapheme_break(before);
    let after_gb = sanedit_ucd::grapheme_break(after);

    // TODO investigate performance if these are in a table?
    match (before_gb, after_gb) {
        (CR, LF) => NoBreak,              // GB 3
        (Control | CR | LF, _) => Break,  // GB 4
        (_, Control | CR | LF) => Break,  // GB 5
        (L, L | V | LV | LVT) => NoBreak, // GB 6
        (LV | V, V | T) => NoBreak,       // GB 7
        (LVT | T, T) => NoBreak,          // GB 8
        (_, Extend | ZWJ) => NoBreak,     // GB 9
        (_, SpacingMark) => NoBreak,      // GB 9a
        (Prepend, _) => NoBreak,          // GB 9b
        (ZWJ, _) => {
            // GB 11
            if Property::ExtendedPictographic.check(after) {
                Emoji
            } else {
                NoBreak
            }
        }
        (RegionalIndicator, RegionalIndicator) => Regional, // GB12, GB13
        (_, _) => Break,                                    // GB 999
    }
}

fn is_break_emoji(iter: &mut Chars) -> bool {
    while let Some((_, _, ch)) = iter.prev() {
        match sanedit_ucd::grapheme_break(ch) {
            GraphemeBreak::Extend => {}
            _ => return !Property::ExtendedPictographic.check(ch),
        }
    }

    true
}

fn is_break_regional(iter: &mut Chars) -> bool {
    let mut ri_count = 0;
    while let Some((_, _, ch)) = iter.prev() {
        match sanedit_ucd::grapheme_break(ch) {
            GraphemeBreak::RegionalIndicator => ri_count += 1,
            _ => return (ri_count % 2) != 0,
        }
    }

    true
}

enum BreakResult {
    Break,
    NoBreak,

    /// Do not break within emoji modifier sequences or emoji zwj sequences.
    /// GB11    \p{Extended_Pictographic} Extend* ZWJ   ×   \p{Extended_Pictographic}
    Emoji,

    /// Do not break within emoji flag sequences. That is, do not break between regional indicator (RI) symbols if there is an odd number of RI characters before the break point.
    /// GB12    sot (RI RI)* RI     ×   RI
    /// GB13    [^RI] (RI RI)* RI   ×   RI
    Regional,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::piece_tree::PieceTree;

    #[test]
    fn next_grapheme_boundary_multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert_str(0, CONTENT);

        let boundaries = [3, 7, 11, 17, 20, 22, 25, 28, 31, 34, 37, 48, 56, 56];
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

        let boundaries = [1, 2, 6, 12, 15];
        let mut pos = 0;
        let slice = pt.slice(5..20);

        for boundary in boundaries {
            pos = next_grapheme_boundary(&slice, pos);
            assert_eq!(boundary, pos);
        }
    }

    #[test]
    fn prev_grapheme_boundary_multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert_str(0, CONTENT);

        let boundaries = [0, 0, 3, 7, 11, 17, 20, 22, 25, 28, 31, 34, 37, 48];
        let slice = pt.slice(..);
        let mut pos = slice.len();

        for boundary in boundaries.iter().rev() {
            pos = prev_grapheme_boundary(&slice, pos);
            assert_eq!(*boundary, pos);
        }
    }

    #[test]
    fn prev_grapheme_boundary_multi_byte_slice() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄";
        pt.insert_str(0, CONTENT);

        let boundaries = [0, 0, 1, 2, 6, 12];
        let slice = pt.slice(5..20);
        let mut pos = slice.len();

        for boundary in boundaries.iter().rev() {
            pos = prev_grapheme_boundary(&slice, pos);
            assert_eq!(*boundary, pos);
        }
    }
}
