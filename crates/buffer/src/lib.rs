mod cursor_iterator;
mod piece_tree;
mod search;

pub use piece_tree::{
    builder::PieceTreeBuilder, bytes::Bytes, chunks::Chunks, slice::PieceTreeSlice, Mark,
    PieceTree, Snapshot,
};

pub use search::{SearchIter, Searcher};

pub mod utf8 {
    use super::*;

    pub use piece_tree::utf8::{
        chars::Chars,
        graphemes::{next_grapheme, next_grapheme_boundary, prev_grapheme, prev_grapheme_boundary},
    };
}
