#[macro_use]
extern crate lazy_static;

mod cursor_iterator;
mod piece_tree;
mod search;

pub use piece_tree::{
    builder::PieceTreeBuilder, bytes::Bytes, chunks::Chunks, slice::PieceTreeSlice, Mark,
    PieceTree, ReadOnlyPieceTree,
};

pub use search::{SearchIter, SearchIterRev, Searcher, SearcherRev};

pub mod utf8 {
    use super::*;

    pub use piece_tree::utf8::{
        chars::Chars,
        graphemes::{next_grapheme, next_grapheme_boundary, prev_grapheme, prev_grapheme_boundary},
        lines::{next_eol, prev_eol, EOLMatch, Lines},
    };
}
