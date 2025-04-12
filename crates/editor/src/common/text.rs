use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{grapheme_category, GraphemeCategory};

pub(crate) fn is_eol_or_eof_at(slice: &PieceTreeSlice, pos: u64) -> bool {
    let mut graphemes = slice.graphemes_at(pos);
    graphemes.next().map(|g| g.is_eol()).unwrap_or(true)
}

/// returns the line start if only whitespace between linestart and pos. also
/// returns none if no whitespace on line at all
pub(crate) fn only_whitespace_before(slice: &PieceTreeSlice, pos: u64) -> Option<u64> {
    let mut graphemes = slice.graphemes_at(pos);
    while let Some(g) = graphemes.prev() {
        if g.is_eol() {
            // If no whitespace return none
            if g.end() == pos {
                return None;
            }

            return Some(g.end());
        }

        let cat = grapheme_category(&g);
        if !matches!(cat, GraphemeCategory::Whitespace) {
            return None;
        }
    }

    None
}
