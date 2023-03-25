use crate::piece_tree::PieceTreeSlice;

pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut chars = slice.chars_at(pos);
    let current = chars.next();
    let after = chars.next();

    match (current, after) {
        (Some((_, _, a)), Some((_, _, b))) => {
            todo!()
        }
        (None, None) => slice.end(),
        (Some(_), None) => slice.end(),
        (None, Some(_)) => unreachable!(),
    }
}

// https://www.unicode.org/reports/tr29/
//
// Break at the start and end of text, unless the text is empty.
// GB1     sot     ÷   Any
// GB2     Any     ÷   eot
// Do not break between a CR and LF. Otherwise, break before and after controls.
// GB3     CR  ×   LF
// GB4     (Control | CR | LF)     ÷
// GB5         ÷   (Control | CR | LF)
// Do not break Hangul syllable sequences.
// GB6     L   ×   (L | V | LV | LVT)
// GB7     (LV | V)    ×   (V | T)
// GB8     (LVT | T)   ×   T
// Do not break before extending characters or ZWJ.
// GB9         ×   (Extend | ZWJ)
// The GB9a and GB9b rules only apply to extended grapheme clusters:
// Do not break before SpacingMarks, or after Prepend characters.
// GB9a        ×   SpacingMark
// GB9b    Prepend     ×
// Do not break within emoji modifier sequences or emoji zwj sequences.
// GB11    \p{Extended_Pictographic} Extend* ZWJ   ×   \p{Extended_Pictographic}
// Do not break within emoji flag sequences. That is, do not break between regional indicator (RI) symbols if there is an odd number of RI characters before the break point.
// GB12    sot (RI RI)* RI     ×   RI
// GB13    [^RI] (RI RI)* RI   ×   RI
// Otherwise, break everywhere.
// GB999   Any     ÷   Any
fn is_break(before: char, after: char) -> BreakResult {
    use sanedit_ucd::GraphemeBreak::*;
    use BreakResult::*;

    let before = sanedit_ucd::grapheme_break(before);
    let after = sanedit_ucd::grapheme_break(after);

    // TODO ascii performance improvement?
    // TODO investigate performance if these are in a table?
    match (before, after) {
        (CR, LF) => NoBreak,              // GB 3
        (Control | CR | LF, _) => Break,  // GB 4
        (_, Control | CR | LF) => Break,  // GB 5
        (L, L | V | LV | LVT) => NoBreak, // GB 6
        (LV | V, V | T) => NoBreak,       // GB 7
        (LVT | T, T) => NoBreak,          // GB 8
        (_, Extend | ZWJ) => NoBreak,     // GB 9
        (_, SpacingMark) => NoBreak,      // GB 9a
        (Prepend, _) => NoBreak,          // GB 9b
        // GB 11
        // GB 12
        // GB 13
        (_, _) => Break, // GB 999
    }
}

enum BreakResult {
    Break,
    NoBreak,
}
