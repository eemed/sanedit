use sanedit_buffer::piece_tree::{self, next_grapheme, prev_grapheme, PieceTreeSlice};

use crate::common::char::grapheme_category;
use crate::editor::windows::Cursor;

use super::char::{is_word_break, Char, DisplayOptions, GraphemeCategory};
use super::eol::EOL;

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

pub(crate) fn next_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut prev: Option<GraphemeCategory> = None;

    while let Some(g) = next_grapheme(slice, pos) {
        let string = String::from(&g);
        let cat = grapheme_category(&string);

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

pub(crate) fn prev_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut cat: Option<GraphemeCategory> = None;

    while let Some(g) = prev_grapheme(slice, pos) {
        let string = String::from(&g);
        let prev = grapheme_category(&string);

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

pub(crate) fn width_at_pos(slice: &PieceTreeSlice, pos: usize, opts: &DisplayOptions) -> usize {
    let target = pos;
    let mut pos = start_of_line(slice, pos);
    let mut col = 0;

    while let Some(g) = next_grapheme(&slice, pos) {
        let mut ch = Char::new(&g, col, opts);
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

    while let Some(g) = next_grapheme(&slice, pos) {
        let mut ch = Char::new(&g, col, opts);
        if col + ch.width() > width {
            break;
        }
        if EOL::is_eol(&g) {
            break;
        }
        col += ch.width();
        pos += ch.grapheme_len();
    }

    pos
}
