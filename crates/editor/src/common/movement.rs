use std::ops::Range;

use sanedit_buffer::piece_tree::{self, next_grapheme, prev_grapheme, PieceTreeSlice};

use crate::common::char::grapheme_category;
use crate::editor::windows::Cursor;

use super::char::{is_word_break, is_word_break_end, Char, DisplayOptions, GraphemeCategory};
use super::eol::EOL;
use super::text::{pos_at_width, width_at_pos};

pub(crate) fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    piece_tree::next_grapheme_boundary(slice, pos)
}

pub(crate) fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    piece_tree::prev_grapheme_boundary(slice, pos)
}

pub(crate) fn start_of_line(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut grapheme = piece_tree::prev_grapheme(slice, pos);
    while let Some(g) = grapheme {
        if EOL::is_eol(&g) {
            return pos;
        }
        pos -= g.len();
        grapheme = piece_tree::prev_grapheme(slice, pos);
    }

    pos
}

pub(crate) fn end_of_line(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut grapheme = piece_tree::next_grapheme(slice, pos);
    while let Some(g) = grapheme {
        if EOL::is_eol(&g) {
            return pos;
        }
        pos += g.len();
        grapheme = piece_tree::next_grapheme(slice, pos);
    }

    pos
}

pub(crate) fn next_line_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let start = pos;

    while let Some(g) = next_grapheme(slice, pos) {
        pos += g.len();
        let eol = EOL::is_eol(&g);

        if eol {
            return pos;
        }
    }

    start_of_line(slice, start)
}

pub(crate) fn prev_line_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    while let Some(g) = prev_grapheme(slice, pos) {
        pos -= g.len();
        let eol = EOL::is_eol(&g);

        if eol {
            break;
        }
    }

    start_of_line(slice, pos)
}

/// Find next word start, this will move even if we currently are on a word
/// start.
pub(crate) fn next_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut prev: Option<GraphemeCategory> = None;

    while let Some(g) = next_grapheme(slice, pos) {
        let cat = grapheme_category(&g);

        if let Some(ref prev) = prev {
            if is_word_break(prev, &cat) {
                return pos;
            }
        }

        pos += g.len();
        prev = Some(cat);
    }

    slice.len()
}

/// Find previous word start, this will move even if we currently are on a word
/// start.
pub(crate) fn prev_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut cat: Option<GraphemeCategory> = None;

    while let Some(g) = prev_grapheme(slice, pos) {
        let prev = grapheme_category(&g);

        if let Some(cat) = cat {
            if is_word_break(&prev, &cat) {
                return pos;
            }
        }

        pos -= g.len();
        cat = Some(prev);
    }

    0
}

pub(crate) fn next_word_end(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut prev: Option<(GraphemeCategory, usize)> = None;
    pos = next_grapheme_boundary(slice, pos);

    while let Some(g) = next_grapheme(slice, pos) {
        let cat = grapheme_category(&g);

        if let Some((ref prev, len)) = prev {
            if is_word_break_end(prev, &cat) {
                return pos - len;
            }
        }

        pos += g.len();
        prev = Some((cat, g.len()));
    }

    slice.len()
}

pub(crate) fn prev_word_end(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut cat: Option<GraphemeCategory> = None;
    if let Some(g) = prev_grapheme(slice, pos) {
        pos += g.len();
    }

    while let Some(g) = prev_grapheme(slice, pos) {
        let prev = grapheme_category(&g);
        pos -= g.len();

        if let Some(cat) = cat {
            if is_word_break_end(&prev, &cat) {
                return pos;
            }
        }

        cat = Some(prev);
    }

    0
}

pub(crate) fn next_paragraph(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    pos = start_of_line(slice, pos);

    while let Some(g) = next_grapheme(slice, pos) {
        let eol = EOL::is_eol(&g);
        if !eol {
            break;
        }
        pos += g.len();
    }

    next_blank_line(slice, pos)
}

pub(crate) fn prev_paragraph(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    while let Some(g) = prev_grapheme(slice, pos) {
        let eol = EOL::is_eol(&g);
        if !eol {
            break;
        }
        pos -= g.len();
    }

    prev_blank_line(slice, pos)
}

pub(crate) fn next_blank_line(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    pos = next_line_start(slice, pos);

    while let Some(g) = next_grapheme(slice, pos) {
        let eol = EOL::is_eol(&g);
        if eol {
            return pos;
        }
        pos = next_line_start(slice, pos);
    }

    slice.len()
}

pub(crate) fn prev_blank_line(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    pos = prev_line_start(slice, pos);

    while let Some(g) = next_grapheme(slice, pos) {
        let eol = EOL::is_eol(&g);
        if eol || pos == 0 {
            return pos;
        }
        pos = prev_line_start(slice, pos);
    }

    0
}

pub(crate) fn next_line(
    slice: &PieceTreeSlice,
    cursor: &Cursor,
    opts: &DisplayOptions,
) -> (usize, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = next_line_start(slice, cpos);
    let npos = pos_at_width(slice, pos, width, opts);
    (npos, width)
}

pub(crate) fn prev_line(
    slice: &PieceTreeSlice,
    cursor: &Cursor,
    opts: &DisplayOptions,
) -> (usize, usize) {
    let cpos = cursor.pos();
    let width = cursor
        .column()
        .unwrap_or_else(|| width_at_pos(slice, cpos, opts));
    let pos = prev_line_start(slice, cpos);
    let npos = pos_at_width(slice, pos, width, opts);
    (npos, width)
}
