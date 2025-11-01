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

pub(crate) fn trim_whitespace(slice: &PieceTreeSlice) -> PieceTreeSlice {
    let mut n = 0;
    let mut graphemes = slice.graphemes();
    while let Some(g) = graphemes.next() {
        let cat = grapheme_category(&g);
        if matches!(cat, GraphemeCategory::Whitespace) {
            n += g.len();
        } else {
            break;
        }
    }

    slice.slice(n..)
}

pub(crate) fn trim_whitespace_back(slice: &PieceTreeSlice) -> PieceTreeSlice {
    let mut n = 0;
    let mut graphemes = slice.graphemes_at(slice.len());
    while let Some(g) = graphemes.prev() {
        let cat = grapheme_category(&g);
        if matches!(cat, GraphemeCategory::Whitespace) {
            n += g.len();
        } else {
            break;
        }
    }

    slice.slice(..slice.len() - n)
}

pub(crate) fn trim_comment_on_line(
    line: &PieceTreeSlice,
    comment: &str,
) -> Option<PieceTreeSlice> {
    let line = trim_whitespace(line);
    if line.len() < comment.len() as u64 {
        return None;
    }
    let possible_comment = line.slice(..comment.len() as u64);
    if possible_comment == comment.as_bytes() {
        let line = line.slice(comment.len() as u64..);
        return Some(line);
    }

    None
}

pub(crate) fn trim_comment_on_line_back(
    line: &PieceTreeSlice,
    comment: &str,
) -> Option<PieceTreeSlice> {
    let line = trim_whitespace_back(line);
    if line.len() < comment.len() as u64 {
        return None;
    }
    let possible_comment = line.slice(line.len() - comment.len() as u64..);
    if possible_comment == comment.as_bytes() {
        let line = line.slice(..line.len() - comment.len() as u64);
        return Some(line);
    }

    None
}
