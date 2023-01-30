use sanedit_buffer::piece_tree::{self, PieceTreeSlice};

use super::eol::EOL;

pub(crate) fn next_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    piece_tree::next_grapheme_boundary(slice, pos)
}

pub(crate) fn prev_grapheme_boundary(slice: &PieceTreeSlice, pos: usize) -> usize {
    piece_tree::prev_grapheme_boundary(slice, pos)
}

pub(crate) fn start_of_line(slice: &PieceTreeSlice, mut pos: usize) -> usize {
    log::info!("sol");
    // TODO use bytes for more efficient impl
    let mut grapheme = piece_tree::prev_grapheme(slice, pos);
    while let Some(g) = grapheme {
        if EOL::is_eol(&g) {
            return pos;
        }
        pos -= g.len();
        log::info!("{pos}");
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
    todo!()
}

pub(crate) fn prev_line(slice: &PieceTreeSlice, pos: usize) -> usize {
    todo!()
}

// pub(crate) fn next_word_start(slice: &PieceTreeSlice, pos: usize) -> usize {}
// pub(crate) fn prev_word_start(slice: &PieceTreeSlice, pos: usize) -> usize {}
// pub(crate) fn next_whitespace_word_start(slice: &PieceTreeSlice, pos: usize) -> usize {}
// pub(crate) fn prev_whitespace_word_start(slice: &PieceTreeSlice, pos: usize) -> usize {}
// pub(crate) fn next_paragraph(slice: &PieceTreeSlice, pos: usize) -> usize {}
// pub(crate) fn prev_paragraph(slice: &PieceTreeSlice, pos: usize) -> usize {}
