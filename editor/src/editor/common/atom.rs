use std::{cmp, ops::Range};

use sanedit_buffer::piece_tree::{next_grapheme, PieceTreeSlice};
use smallvec::SmallVec1;
use smartstring::alias::String;

/// Representation of a grapheme in the buffer.
/// This is a separate type to distinguish graphemes that have already been
/// converted to the format we want the user to see.
#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub(crate) struct Atom {
    display: String,
    slice_range: Range<usize>,
}

impl Atom {}

impl std::ops::Deref for Atom {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

/// Options on how to display atoms
#[derive(Debug, Clone)]
pub(crate) struct AtomOptions {
    pub tabstop: u8,
    pub line_width: usize,
}

impl Default for AtomOptions {
    fn default() -> Self {
        AtomOptions {
            tabstop: 8,
            line_width: 80,
        }
    }
}

#[inline]
fn grapheme_width(grapheme: PieceTreeSlice, column: usize, options: &AtomOptions) -> usize {
    grapheme_to_atoms(grapheme, column, options).len()
}

#[inline]
fn grapheme_to_atoms(
    grapheme: PieceTreeSlice,
    column: usize,
    options: &AtomOptions,
) -> SmallVec1<Atom> {
    let mut atoms = SmallVec1::new();
    if grapheme == "\t" {}

    let mut chars = grapheme.chars();
    let mut atom = String::new();
    while let Some((_pos, _, ch)) = chars.next() {
        atom.push(ch);
    }
    atoms.push(Atom {
        display: atom,
        slice_range: grapheme.start()..grapheme.end(),
    });
    atoms
}

#[cfg(test)]
mod test {
    use sanedit_buffer::piece_tree::PieceTree;
    use std::ops::Deref;

    use super::*;

    #[test]
    fn emoji() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "❤️");
        let slice = pt.slice(..);
        let atoms = grapheme_to_atoms(slice, 0, &AtomOptions::default());
        assert_eq!("❤️", atoms[0].deref());
    }

    #[test]
    fn control_sequence_null() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\0");
        let slice = pt.slice(..);
        let atoms = grapheme_to_atoms(slice, 0, &AtomOptions::default());
        assert_eq!("\0", atoms[0].deref());
    }

    #[test]
    fn invalid_utf8() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\xFF\xFF");
        let slice = pt.slice(..);
        let atoms = grapheme_to_atoms(slice, 0, &AtomOptions::default());
        assert_eq!("\u{fffd}", atoms[0].deref());
    }

    #[test]
    fn tab() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "\t");
    }

    #[test]
    fn non_standard_spaces() {
        // TODO non breaking spaces only?
        let mut pt = PieceTree::new();
        pt.insert_str(0, "\u{00A0}");
    }
}
