mod piece_tree;

pub use piece_tree::{
    builder::PieceTreeBuilder,
    bytes::Bytes,
    chunks::{Chunk, Chunks},
    mark::{Mark, MarkResult},
    slice::PieceTreeSlice,
    view::PieceTreeView,
    PieceTree,
};

pub mod utf8 {
    use super::*;

    pub use piece_tree::utf8::{
        chars::{
            decode_utf8, decode_utf8_iter, Chars, ACCEPT, REJECT, UTF8_CHAR_CLASSES,
            UTF8_TRANSITIONS,
        },
        graphemes::{next_grapheme_boundary, prev_grapheme_boundary, Grapheme, Graphemes},
        lines::{next_eol, prev_eol, EOLMatch, EndOfLine, Lines},
    };
}
