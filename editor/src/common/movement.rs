use sanedit_buffer::piece_tree::{self, next_grapheme, prev_grapheme, PieceTreeSlice};

use crate::common::char::grapheme_category;

use super::char::GraphemeCategory;
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

pub(crate) fn next_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    // let mut graphemes = buf.graphemes_at(pos);
    // let at_eol = graphemes.get().map_or(false, |g| is_buf_eol(buf, &g));
    // if at_eol {
    //     return graphemes.pos() + buf.eol.len();
    // }

    // let next_eol = graphemes.find_next(|g| is_buf_eol(&buf, g));
    // next_eol.map_or_else(
    //     || start_of_line(buf, pos),
    //     |_| graphemes.pos() + buf.eol.len(),
    // )
    todo!()
}

pub(crate) fn prev_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    todo!()
}

pub(crate) fn next_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut prev: Option<GraphemeCategory> = None;

    // TODO dont allocate
    while let Some(g) = next_grapheme(slice, pos) {
        let string = String::from(&g);
        let cat = grapheme_category(&string);

        if let Some(ref prev) = prev {
            if cat.is_word_break(prev) {
                return pos;
            }
        }

        pos += g.len();
        prev = Some(cat);
    }

    slice.len()
}

pub(crate) fn prev_word_start(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    let mut next: Option<GraphemeCategory> = None;

    // TODO dont allocate
    while let Some(g) = prev_grapheme(slice, pos) {
        let string = String::from(&g);
        let cat = grapheme_category(&string);

        if let Some(next) = next {
            if next.is_word_break(&cat) {
                return pos;
            }
        }

        pos -= g.len();
        next = Some(cat);
    }

    0
}

// pub(crate) fn next_paragraph(slice: &PieceTreeSlice, pos: usize) -> usize {
// }

// pub(crate) fn prev_paragraph(slice: &PieceTreeSlice, pos: usize) -> usize {
// }
