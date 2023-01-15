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

/// An iterator of atoms from a buffer slice.
#[derive(Debug, Clone)]
pub(crate) struct AtomIterator<'a> {
    /// Sometimes graphemes are segmented into multiple atoms, and we want to
    /// yield them one by one.
    queue: Vec<Atom>,
    slice_offset: usize,
    slice: PieceTreeSlice<'a>,
    line: usize,
    column: usize,
    options: AtomOptions,
}

impl<'a> AtomIterator<'a> {
    pub fn new(
        slice: PieceTreeSlice<'a>,
        line: usize,
        column: usize,
        options: AtomOptions,
    ) -> AtomIterator<'a> {
        AtomIterator {
            queue: Vec::new(),
            slice_offset: 0,
            slice,
            line,
            column,
            options,
        }
    }

    /// Advances one position on the grid, returns the old position
    fn advance(&mut self) -> (usize, usize) {
        let line = self.line;
        let col = self.column;
        if self.column == self.options.line_width {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        (line, col)
    }
}

impl<'a> Iterator for AtomIterator<'a> {
    type Item = (usize, usize, Atom);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(atom) = self.queue.pop() {
            let (line, col) = self.advance();
            return Some((line, col, atom));
        }

        let grapheme = next_grapheme(&self.slice, self.slice_offset)?;
        self.slice_offset += grapheme.len();
        let atoms = grapheme_to_atoms(grapheme, self.column, &self.options);
        let (first, rest) = atoms.split_first()?;
        self.queue.extend_from_slice(rest);
        let (line, col) = self.advance();
        Some((line, col, first.clone()))
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
    while let Some((_pos, ch)) = chars.next() {
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

    use super::*;

    #[test]
    fn emoji() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "❤️");
        let slice = pt.slice(..);

        let mut iter = AtomIterator::new(slice, 0, 0, AtomOptions::default());
        println!("{:?}", iter.next());
    }

    #[test]
    fn control_sequence_null() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\0");
        let slice = pt.slice(..);
        let mut iter = AtomIterator::new(slice, 0, 0, AtomOptions::default());
        println!("{:?}", iter.next());
    }

    #[test]
    fn invalid_utf8() {
        let mut pt = PieceTree::new();
        pt.insert(0, b"\xFF\xFF");
        let slice = pt.slice(..);
        let mut iter = AtomIterator::new(slice, 0, 0, AtomOptions::default());
        println!("{:?}", iter.next());
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
