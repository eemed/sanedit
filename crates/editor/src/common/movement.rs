use sanedit_buffer::piece_tree::{self, next_grapheme, prev_grapheme, PieceTreeSlice};

use crate::common::char::grapheme_category;

use super::char::{is_word_break, GraphemeCategory};
use super::eol::EOL;

pub(crate) fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    piece_tree::next_grapheme_boundary(slice, pos)
}

pub(crate) fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    piece_tree::prev_grapheme_boundary(slice, pos)
}

pub(crate) fn start_of_line(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    // TODO use bytes for more efficient impl
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
    // TODO use bytes for more efficient impl
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

    // TODO dont allocate
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

    // TODO dont allocate
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
