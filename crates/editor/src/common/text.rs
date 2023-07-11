use std::ops::Range;

use sanedit_buffer::{utf8::EndOfLine, PieceTreeSlice};

use crate::common::movement::next_grapheme_boundary;

use super::{
    char::{
        grapheme_category, is_word_break, is_word_break_end, Char, DisplayOptions, GraphemeCategory,
    },
    movement::{next_word_end, prev_word_start, start_of_line},
};

pub(crate) fn width_at_pos(slice: &PieceTreeSlice, pos: usize, opts: &DisplayOptions) -> usize {
    let target = pos;
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let ch = Char::new(&g, col, opts);
        if pos >= target {
            break;
        }

        col += ch.width();
        pos += ch.grapheme_len();
    }

    col
}

/// returns the position at width + the width at the position
pub(crate) fn pos_at_width(
    slice: &PieceTreeSlice,
    pos: usize,
    width: usize,
    opts: &DisplayOptions,
) -> usize {
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;
    let mut graphemes = slice.graphemes_at(pos);

    while let Some(g) = graphemes.next() {
        let ch = Char::new(&g, col, opts);
        if col + ch.width() > width {
            break;
        }
        if EndOfLine::is_slice_eol(&g) {
            break;
        }
        col += ch.width();
        pos += ch.grapheme_len();
    }

    pos
}

pub(crate) fn on_word_end(
    prev: (usize, GraphemeCategory),
    next: Option<(usize, GraphemeCategory)>,
    pos: usize,
    slice_len: usize,
) -> bool {
    match (prev, next) {
        ((_, p), Some((_, n))) => is_word_break_end(&p, &n),
        ((len, _), _) => pos + len == slice_len,
    }
}

pub(crate) fn on_word_start(
    prev: Option<(usize, GraphemeCategory)>,
    next: (usize, GraphemeCategory),
    pos: usize,
) -> bool {
    match (prev, next) {
        (Some((_, p)), (_, n)) => is_word_break(&p, &n),
        _ => pos == 0,
    }
}

/// Returns the range of the word that includes position pos
pub(crate) fn word_at_pos(slice: &PieceTreeSlice, pos: usize) -> Option<Range<usize>> {
    let make_pair = |slice: PieceTreeSlice| {
        let len = slice.len();
        let cat = grapheme_category(&slice);
        (len, cat)
    };
    let mut graphemes = slice.graphemes_at(pos);

    let before = if graphemes.prev().is_none() {
        None
    } else {
        graphemes.next().map(make_pair)
    };
    let current = graphemes.next().map(make_pair)?;
    let after = graphemes.next().map(make_pair);

    if !current.1.is_word() {
        return None;
    }

    let start = {
        let mut start = pos;
        if !on_word_start(before, current, pos) {
            start = prev_word_start(slice, pos);
        }
        start
    };

    let end = {
        let mut end = pos;
        if !on_word_end(current, after, pos, slice.len()) {
            end = next_word_end(slice, pos);
        }
        next_grapheme_boundary(slice, end)
    };

    Some(start..end)

    // let cat = graphemes.next().as_ref().map(grapheme_category)?;
    // if !cat.is_word() {
    //     return None;
    // }

    // let start = if on_word_start(slice, pos) {
    //     pos
    // } else {
    //     prev_word_start(slice, pos)
    // };

    // let end = {
    //     let end = if on_word_end(slice, pos) {
    //         pos
    //     } else {
    //         next_word_end(slice, pos)
    //     };
    //     next_grapheme_boundary(slice, end)
    // };

    // Some(start..end)
}
