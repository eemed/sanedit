use sanedit_ucd::{grapheme_break, GraphemeBreak, Property};

use crate::{piece_tree::PieceTreeSlice, ReadOnlyPieceTree};

use super::chars::Chars;

/// Utility function to quickly return the next grapheme boundary
/// If more iterations are needed using the `Graphemes` iterator is more
/// efficient.
#[inline]
pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut graphemes = slice.graphemes_at(pos);
    match graphemes.next() {
        Some(g) => pos + g.len(),
        _ => slice.len(),
    }
}

/// Utility function to quickly return the prev grapheme boundary
/// If more iterations are needed using the `Graphemes` iterator is more
/// efficient.
#[inline]
pub fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    let mut graphemes = slice.graphemes_at(pos);
    match graphemes.prev() {
        Some(g) => pos - g.len(),
        _ => 0,
    }
}

#[derive(Debug, Clone)]
struct Char {
    start: usize,
    end: usize,
    ch: char,
    gbreak: GraphemeBreak,
}

impl Char {
    pub fn new(ch: (usize, usize, char)) -> Char {
        Char {
            start: ch.0,
            end: ch.1,
            ch: ch.2,
            gbreak: grapheme_break(ch.2),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Graphemes<'a> {
    slice: PieceTreeSlice<'a>,
    chars: Chars<'a>,
    /// Used for next iteration
    prev: Option<Char>,
    /// Wether we have returned the last element or not
    at_end: bool,

    // Used for prev iteration
    next: Option<Char>,
    /// Wether we have returned the first element or not
    at_start: bool,

    last_call_fwd: Option<bool>,
}

impl<'a> Graphemes<'a> {
    pub(crate) fn new(pt: &'a ReadOnlyPieceTree, at: usize) -> Graphemes<'a> {
        let chars = Chars::new(pt, at);
        Graphemes {
            slice: pt.slice(..),
            chars,
            prev: None,
            next: None,
            at_start: at == 0,
            at_end: at == pt.len(),
            last_call_fwd: None,
        }
    }

    pub(crate) fn new_from_slice(slice: &PieceTreeSlice<'a>, at: usize) -> Graphemes<'a> {
        debug_assert!(
            slice.len() >= at,
            "Attempting to index {} over slice len {} ",
            at,
            slice.len(),
        );
        let chars = Chars::new_from_slice(slice, at);
        Graphemes {
            slice: slice.clone(),
            chars,
            prev: None,
            next: None,
            at_start: at == 0,
            at_end: at == slice.len(),
            last_call_fwd: None,
        }
    }

    pub fn next(&mut self) -> Option<PieceTreeSlice> {
        if !self.at_start && self.last_call_fwd == Some(false) {
            self.chars.next();
        }
        self.last_call_fwd = Some(true);
        self.at_start = false;

        let mut current = self
            .prev
            .take()
            .or_else(|| self.chars.next().map(Char::new));
        let mut after = self.chars.next().map(Char::new);
        let start = current.as_ref().map(|c| c.start).unwrap_or(0);

        loop {
            match (current, after) {
                (Some(c), Some(a)) => {
                    if is_break(&self.slice, &c, &a) {
                        let range = start..a.start;
                        self.prev = Some(a);
                        return Some(self.slice.slice(range));
                    }

                    current = Some(a);
                    after = self.chars.next().map(Char::new);
                }
                (Some(_), None) => {
                    if self.at_end {
                        return None;
                    }

                    self.prev = None;
                    self.at_end = true;
                    return Some(self.slice.slice(start..self.slice.len()));
                }
                (None, None) => return None,
                (None, Some(_)) => unreachable!(),
            }
        }
    }

    pub fn prev(&mut self) -> Option<PieceTreeSlice> {
        if self.last_call_fwd == Some(true) {
            self.chars.prev();
            self.prev = None;
        }
        self.last_call_fwd = Some(false);
        self.at_end = false;

        let mut after = self
            .next
            .take()
            .or_else(|| self.chars.prev().map(Char::new));

        let mut current = self.chars.prev().map(Char::new);
        let end = after.as_ref().map(|a| a.end).unwrap_or(self.slice.len());

        loop {
            match (current, after) {
                (Some(c), Some(a)) => {
                    if is_break(&self.slice, &c, &a) {
                        let range = a.start..end;
                        self.next = Some(c);
                        return Some(self.slice.slice(range));
                    }

                    after = Some(c);
                    current = self.chars.prev().map(Char::new);
                }
                (None, Some(_)) => {
                    if self.at_start {
                        return None;
                    }

                    self.next = None;
                    self.at_start = true;
                    return Some(self.slice.slice(0..end));
                }
                (None, None) => return None,
                (Some(_), None) => unreachable!(),
            }
        }
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

fn is_break(slice: &PieceTreeSlice, before: &Char, after: &Char) -> bool {
    use BreakResult::*;

    match pair_break(before, after) {
        Break => true,
        Emoji => {
            let mut before = slice.chars_at(before.start);
            is_break_emoji(&mut before)
        }
        Regional => {
            let mut before = slice.chars_at(before.start);
            is_break_regional(&mut before)
        }
        NoBreak => false,
    }
}

// https://www.unicode.org/reports/tr29/
/// Check if a grapheme break exists between the two characters
fn pair_break(before: &Char, after: &Char) -> BreakResult {
    use sanedit_ucd::GraphemeBreak::*;
    use BreakResult::*;

    match (before.gbreak, after.gbreak) {
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
            if Property::ExtendedPictographic.check(after.ch) {
                Emoji
            } else {
                NoBreak
            }
        }
        (RegionalIndicator, RegionalIndicator) => Regional, // GB12, GB13
        (_, _) => Break,                                    // GB 999
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::piece_tree::PieceTree;

    macro_rules! assert_str {
        ($expect:expr, $slice:expr) => {{
            let slice = $slice.expect("No grapheme present");
            let actual = String::from(&slice);
            assert_eq!($expect, actual);
        }};
    }

    #[test]
    fn grapheme_iter_next() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert(0, CONTENT);

        let boundaries = [3, 7, 11, 17, 20, 22, 25, 28, 31, 34, 37, 48, 56, 56];
        let slice = pt.slice(..);
        let mut graphemes = slice.graphemes_at(0);
        let mut pos = 0;

        for boundary in boundaries {
            if let Some(g) = graphemes.next() {
                pos += g.len();
            }
            assert_eq!(boundary, pos);
        }
    }

    #[test]
    fn grapheme_iter_prev() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert(0, CONTENT);

        let boundaries = [0, 0, 3, 7, 11, 17, 20, 22, 25, 28, 31, 34, 37, 48];
        let slice = pt.slice(..);
        let mut graphemes = slice.graphemes_at(slice.len());
        let mut pos = slice.len();

        for boundary in boundaries.iter().rev() {
            if let Some(g) = graphemes.prev() {
                pos -= g.len();
            }
            assert_eq!(*boundary, pos);
        }
    }

    #[test]
    fn grapheme_iter_next_prev() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert(0, CONTENT);

        let slice = pt.slice(..);
        let mut graphemes = slice.graphemes_at(0);

        assert_str!("❤", graphemes.next());
        assert_str!("🤍", graphemes.next());
        assert_str!("🥳", graphemes.next());
        assert_str!("🥳", graphemes.prev());
        assert_str!("🤍", graphemes.prev());
        assert_str!("🤍", graphemes.next());
        assert_str!("🥳", graphemes.next());
    }

    #[test]
    fn grapheme_iter_prev_next() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "abba";
        pt.insert(0, CONTENT);

        let slice = pt.slice(..);
        let mut graphemes = slice.graphemes_at(slice.len());

        assert_str!("a", graphemes.prev());
        assert_str!("b", graphemes.prev());
        assert_str!("b", graphemes.prev());
        assert_str!("a", graphemes.prev());
        assert!(graphemes.prev().is_none());
        assert!(graphemes.prev().is_none());
        assert_str!("a", graphemes.next());
    }

    #[test]
    fn grapheme_iter_middle() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert(0, CONTENT);
        let slice = pt.slice(..);
        let mut graphemes = slice.graphemes_at(11);

        assert_str!("❤️", graphemes.next());
    }

    #[test]
    fn next_grapheme_boundary_multi_byte() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "❤🤍🥳❤️간÷나는산다⛄😮‍💨🇫🇮";
        pt.insert(0, CONTENT);

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
        pt.insert(0, CONTENT);

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
        pt.insert(0, CONTENT);

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
        pt.insert(0, CONTENT);

        let boundaries = [0, 0, 1, 2, 6, 12];
        let slice = pt.slice(5..20);
        let mut pos = slice.len();

        for boundary in boundaries.iter().rev() {
            pos = prev_grapheme_boundary(&slice, pos);
            assert_eq!(*boundary, pos);
        }
    }

    #[test]
    fn iter_prev_next() {
        let mut pt = PieceTree::new();
        const CONTENT: &str = "abba";
        pt.insert(0, CONTENT);

        let boundaries = [0, 0, 1, 2, 3];
        let slice = pt.slice(..);
        let mut pos = slice.len();

        for boundary in boundaries.iter().rev() {
            pos = prev_grapheme_boundary(&slice, pos);
            assert_eq!(*boundary, pos);
        }

        pos = next_grapheme_boundary(&slice, pos);
        assert_eq!(1, pos);
    }
}
