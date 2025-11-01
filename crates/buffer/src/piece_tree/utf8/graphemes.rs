use std::{fmt::Display, ops::Range};

use sanedit_ucd::{grapheme_break, GraphemeBreak, Property};

use crate::{
    piece_tree::{utf8::chars::REPLACEMENT_CHAR, PieceTreeSlice},
    utf8::{decode_utf8, EndOfLine},
    Chunk,
};

use super::chars::Chars;

/// Utility function to quickly return the next grapheme boundary
/// If more iterations are needed using the `Graphemes` iterator is more
/// efficient.
#[inline]
pub fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: u64) -> u64 {
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
pub fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: u64) -> u64 {
    let mut graphemes = slice.graphemes_at(pos);
    match graphemes.prev() {
        Some(g) => pos - g.len(),
        _ => 0,
    }
}

#[derive(Debug)]
pub enum Grapheme<'a> {
    Ref {
        chunk_start: u64,
        chunk: Chunk<'a>,
        range: Range<u64>,
    },
    Owned {
        start: u64,
        text: Vec<u8>,
    },
}

impl<'a> Grapheme<'a> {
    pub fn len(&self) -> u64 {
        match self {
            Grapheme::Ref { range, .. } => range.end - range.start,
            Grapheme::Owned { text, .. } => text.len() as u64,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn start(&self) -> u64 {
        match self {
            Grapheme::Ref {
                chunk_start, range, ..
            } => chunk_start + range.start,
            Grapheme::Owned { start, .. } => *start,
        }
    }

    pub fn end(&self) -> u64 {
        match self {
            Grapheme::Ref {
                chunk_start, range, ..
            } => chunk_start + range.end,
            Grapheme::Owned { start, text } => *start + text.len() as u64,
        }
    }

    pub fn is_eol(&self) -> bool {
        EndOfLine::is_eol(self.as_ref())
    }
}

impl<'a> Display for Grapheme<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut bytes = self.as_ref();
        while !bytes.is_empty() {
            let (ch, n) = decode_utf8(bytes);
            let ch = ch.unwrap_or(REPLACEMENT_CHAR);
            write!(f, "{}", ch)?;
            bytes = &bytes[n..];
        }

        Ok(())
    }
}

impl<'a> AsRef<[u8]> for Grapheme<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Grapheme::Ref { chunk, range, .. } => {
                &chunk.as_ref()[range.start as usize..range.end as usize]
            }
            Grapheme::Owned { text, .. } => text,
        }
    }
}

impl<'a> PartialEq<&str> for Grapheme<'a> {
    fn eq(&self, other: &&str) -> bool {
        other.as_bytes() == self.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct Graphemes<'a> {
    slice: &'a PieceTreeSlice,
    chars: Chars<'a>,
    /// Used for next iteration
    prev: Option<GbChar>,
    /// Wether we have returned the last element or not
    at_end: bool,

    // Used for prev iteration
    next: Option<GbChar>,
    /// Wether we have returned the first element or not
    at_start: bool,

    last_call_fwd: Option<bool>,
}

impl<'a> Graphemes<'a> {
    pub(crate) fn new(slice: &PieceTreeSlice, at: u64) -> Graphemes<'_> {
        debug_assert!(
            slice.len() >= at,
            "Attempting to index {} over slice len {} ",
            at,
            slice.len(),
        );
        let chars = Chars::new(slice, at);
        Graphemes {
            slice,
            chars,
            prev: None,
            next: None,
            at_start: at == 0,
            at_end: at == slice.len(),
            last_call_fwd: None,
        }
    }

    pub fn next_slice(&mut self) -> Option<PieceTreeSlice> {
        if !self.at_start && self.last_call_fwd == Some(false) {
            self.chars.next();
        }
        self.last_call_fwd = Some(true);
        self.at_start = false;

        let mut current = self
            .prev
            .take()
            .or_else(|| self.chars.next().map(GbChar::new));
        let mut after = self.chars.next().map(GbChar::new);
        let start = current.as_ref().map(|c| c.start).unwrap_or(0);

        loop {
            match (current, after) {
                (Some(c), Some(a)) => {
                    if is_break(self.slice, &c, &a) {
                        let range = start..a.start;
                        self.prev = Some(a);
                        return Some(self.slice.slice(range));
                    }

                    current = Some(a);
                    after = self.chars.next().map(GbChar::new);
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

    pub fn prev_slice(&mut self) -> Option<PieceTreeSlice> {
        if self.last_call_fwd == Some(true) {
            self.chars.prev();
            self.prev = None;
        }
        self.last_call_fwd = Some(false);
        self.at_end = false;

        let mut after = self
            .next
            .take()
            .or_else(|| self.chars.prev().map(GbChar::new));

        let mut current = self.chars.prev().map(GbChar::new);
        let end = after.as_ref().map(|a| a.end).unwrap_or(self.slice.len());

        loop {
            match (current, after) {
                (Some(c), Some(a)) => {
                    if is_break(self.slice, &c, &a) {
                        let range = a.start..end;
                        self.next = Some(c);
                        return Some(self.slice.slice(range));
                    }

                    after = Some(c);
                    current = self.chars.prev().map(GbChar::new);
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

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Grapheme<'a>> {
        let prev_chunk = self.chars.current_chunk();
        let slice = self.next_slice()?;
        let next_chunk = self.chars.current_chunk();

        let chunks = [prev_chunk, next_chunk];
        for (start, chk) in chunks.into_iter().flatten() {
            let chunk = chk.as_ref();
            let end = start + chunk.len() as u64;
            if start <= slice.start() && slice.end() <= end {
                let chk_start = slice.start() - start;
                return Some(Grapheme::Ref {
                    chunk_start: start,
                    chunk: chk,
                    range: chk_start..chk_start + slice.len(),
                });
            }
        }

        Some(Grapheme::Owned {
            start: slice.start(),
            text: Vec::from(&slice),
        })
    }

    pub fn prev(&mut self) -> Option<Grapheme<'a>> {
        let next_chunk = self.chars.current_chunk();
        let slice = self.prev_slice()?;
        let prev_chunk = self.chars.current_chunk();

        let chunks = [prev_chunk, next_chunk];
        for (start, chk) in chunks.into_iter().flatten() {
            let chunk = chk.as_ref();
            let end = start + chunk.len() as u64;
            if start <= slice.start() && slice.end() <= end {
                let chk_start = slice.start() - start;
                return Some(Grapheme::Ref {
                    chunk_start: start,
                    chunk: chk,
                    range: chk_start..chk_start + slice.len(),
                });
            }
        }

        Some(Grapheme::Owned {
            start: slice.start(),
            text: Vec::from(&slice),
        })
    }
}

/// Grapheme break character. Used to determine a grapheme break.
#[derive(Debug, Clone)]
struct GbChar {
    start: u64,
    end: u64,
    ch: char,
    gbreak: GraphemeBreak,
}

impl GbChar {
    pub fn new(ch: (u64, u64, char)) -> GbChar {
        GbChar {
            start: ch.0,
            end: ch.1,
            ch: ch.2,
            gbreak: grapheme_break(ch.2),
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

fn is_break(slice: &PieceTreeSlice, before: &GbChar, after: &GbChar) -> bool {
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
fn pair_break(before: &GbChar, after: &GbChar) -> BreakResult {
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
            let actual = std::str::from_utf8(slice.as_ref()).unwrap();
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
