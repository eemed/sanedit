use sanedit_ucd::Property;

use crate::piece_tree::PieceTreeSlice;

pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    use BreakResult::*;

    let mut chars = slice.chars_at(pos);
    let mut current = chars.next();
    let mut after = chars.next();

    loop {
        match (current, after) {
            (Some((_, _, a)), Some((start, _, b))) => {
                match is_break(a, b) {
                    Break => start,
                    Emoji => todo!(),
                    Regional => todo!(),
                    NoBreak => todo!(),
                }

                // Progress forward
                current = after;
                after = chars.next();
            }
            (None, None) => slice.end(),
            (Some(_), None) => slice.end(),
            (None, Some(_)) => unreachable!(),
        }
    }
}

// https://www.unicode.org/reports/tr29/
/// Check if a grapheme break exists between the two characters
fn is_break(before: char, after: char) -> BreakResult {
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
            let is_ext_pic = Property::ExtendedPictographic.check(after);
            if is_ext_pic {
                Emoji
            } else {
                NoBreak
            }
        }
        (RegionalIndicator, RegionalIndicator) => Regional, // GB12, GB13
        (_, _) => Break,                                    // GB 999
    }
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
