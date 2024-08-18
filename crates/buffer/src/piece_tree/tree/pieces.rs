use std::ops::Range;

use super::node::internal_node::InternalNode;
use super::node::Node;
use super::piece::Piece;
use crate::piece_tree::ReadOnlyPieceTree;
use crate::PieceTreeSlice;

pub(crate) type Pieces<'a> = BoundedPieceIter<'a>;

/// Piece iterator that can be bounded to a buffer range
#[derive(Debug, Clone)]
pub(crate) struct BoundedPieceIter<'a> {
    range: Range<u64>,
    iter: PieceIter<'a>,
}

impl<'a> BoundedPieceIter<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a ReadOnlyPieceTree, at: u64) -> BoundedPieceIter<'a> {
        let iter = PieceIter::new(pt, at);
        BoundedPieceIter {
            range: 0..pt.len(),
            iter,
        }
    }
    #[inline]
    pub(crate) fn new_from_slice(slice: &PieceTreeSlice<'a>, at: u64) -> BoundedPieceIter<'a> {
        let iter = PieceIter::new(slice.pt, slice.start() + at);
        BoundedPieceIter {
            range: slice.range.clone(),
            iter,
        }
    }

    #[inline]
    fn shrink_to_range(&self, (mut p_start, mut piece): (u64, Piece)) -> Option<(u64, Piece)> {
        let Range { start, end } = self.range;
        let p_end = p_start + piece.len;

        // Shrink the piece if bounds are met
        if p_start < start {
            let diff = start - p_start;
            piece.split_right(diff);
            p_start += diff;
        }

        if end < p_end {
            piece.split_left(piece.len.saturating_sub(p_end - end));
        }

        if piece.len == 0 {
            return None;
        }

        Some((p_start - start, piece))
    }

    #[inline]
    pub fn get(&self) -> Option<(u64, Piece)> {
        let pos_piece = self.iter.get()?;
        self.shrink_to_range(pos_piece)
    }

    #[inline]
    pub fn next(&mut self) -> Option<(u64, Piece)> {
        let (p_start, _) = self.iter.get()?;
        let Range { end, .. } = self.range;
        let over_bounds = end < p_start;

        if over_bounds {
            return None;
        }

        let pos_piece = self.iter.next()?;
        self.shrink_to_range(pos_piece)
    }

    #[inline]
    pub fn prev(&mut self) -> Option<(u64, Piece)> {
        if let Some((p_start, _)) = self.iter.get() {
            let Range { start, .. } = self.range;
            let over_bounds = p_start <= start;

            if over_bounds {
                return None;
            }
        }

        let pos_piece = self.iter.prev()?;
        self.shrink_to_range(pos_piece)
    }
}

/// Traverse pieces in the tree, in order
#[derive(Debug, Clone)]
pub(crate) struct PieceIter<'a> {
    pt: &'a ReadOnlyPieceTree,
    stack: Vec<&'a InternalNode>,
    pos: u64, // Current piece pos in buffer
}

impl<'a> PieceIter<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a ReadOnlyPieceTree, at: u64) -> Self {
        // log::info!("new pieces at {at}");
        // Be empty at pt.len
        let (stack, pos) = if at == pt.len {
            (Vec::with_capacity(pt.tree.max_height()), at)
        } else {
            pt.tree.find_node(at)
        };
        PieceIter { pt, stack, pos }
    }

    pub fn tree_next(&mut self) -> Option<&Piece> {
        let mut node = *self.stack.last()?;

        // Try to go right
        if let Node::Internal(right) = node.right.as_ref() {
            self.stack.push(right);

            node = right;

            while let Node::Internal(left) = node.left.as_ref() {
                self.stack.push(left);
                node = left;
            }

            Some(&node.piece)
        } else {
            self.stack.pop()?;

            while !self.stack.is_empty() {
                let left = self.stack.last()?.left.as_ref();

                // If we came from left
                if left
                    .internal_ref()
                    .map_or(false, |left| std::ptr::eq(left, node))
                {
                    return Some(&self.stack.last()?.piece);
                }

                node = self.stack.pop()?;
            }

            None
        }
    }

    fn tree_prev(&mut self) -> Option<&Piece> {
        let mut node = *self.stack.last()?;

        // Try to go left
        if let Node::Internal(left) = node.left.as_ref() {
            self.stack.push(left);

            node = left;

            while let Node::Internal(right) = node.right.as_ref() {
                self.stack.push(right);
                node = right;
            }

            Some(&node.piece)
        } else {
            self.stack.pop()?;

            while !self.stack.is_empty() {
                let right = self.stack.last()?.right.as_ref();

                // If we came from right
                if right
                    .internal_ref()
                    .map_or(false, |right| std::ptr::eq(right, node))
                {
                    return Some(&self.stack.last()?.piece);
                }

                node = self.stack.pop()?;
            }

            None
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Option<(u64, Piece)> {
        let piece = self.stack.last().map(|&node| node.piece.clone())?;
        let pos = self.pos();
        Some((pos, piece))
    }

    #[inline]
    pub fn next(&mut self) -> Option<(u64, Piece)> {
        let prev_len = self.get()?.1.len;

        if let Some(p) = self.tree_next().cloned() {
            self.pos += prev_len;
            Some((self.pos, p))
        } else {
            self.pos = self.pt.len;
            None
        }
    }

    #[inline]
    pub fn prev(&mut self) -> Option<(u64, Piece)> {
        if self.pos == 0 {
            return None;
        }

        if let Some(p) = self.tree_prev().cloned() {
            self.pos -= p.len;
            Some((self.pos, p))
        } else {
            let (stack, index) = self.pt.tree.find_node(self.pt.len());
            self.stack = stack;
            self.pos = index;
            self.get()
        }
    }

    #[inline(always)]
    pub fn pos(&self) -> u64 {
        self.pos
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::piece_tree::buffers::BufferKind;
    use crate::PieceTree;

    use super::*;

    fn add_piece(pos: u64, index: u64, len: u64) -> Option<(u64, Piece)> {
        Some((pos, Piece::new(BufferKind::Add, index, len)))
    }

    #[test]
    fn empty() {
        let pt = PieceTree::new();
        let pieces = PieceIter::new(&pt.pt, 0);
        assert_eq!(None, pieces.get());
        assert_eq!(0, pieces.pos());
    }

    #[test]
    fn piece_one() {
        let mut pt = PieceTree::new();
        pt.insert(0, "foobar");
        let mut pieces = PieceIter::new(&pt.pt, 0);

        assert_eq!(add_piece(0, 0, 6), pieces.get());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(0, 0, 6), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 0, 6), pieces.get());
    }

    #[test]
    fn pieces() {
        let mut pt = PieceTree::new();
        pt.insert(0, "baz");
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let mut pieces = PieceIter::new(&pt.pt, 0);

        assert_eq!(add_piece(0, 6, 3), pieces.get());
        assert_eq!(add_piece(3, 3, 3), pieces.next());
        assert_eq!(add_piece(6, 0, 3), pieces.next());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(6, 0, 3), pieces.prev());
        assert_eq!(add_piece(3, 3, 3), pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.get());
    }

    #[test]
    fn at_middle() {
        let mut pt = PieceTree::new();
        pt.insert(0, "baz");
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let mut pieces = PieceIter::new(&pt.pt, 5);

        assert_eq!(add_piece(3, 3, 3), pieces.get());
        assert_eq!(add_piece(6, 0, 3), pieces.next());
        assert_eq!(None, pieces.next());
    }

    #[test]
    fn at_max() {
        let mut pt = PieceTree::new();
        pt.insert(0, "baz");
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let pieces = PieceIter::new(&pt.pt, pt.len());

        assert_eq!(None, pieces.get());
    }

    #[test]
    fn length_1() {
        let mut pt = PieceTree::new();
        pt.insert(0, "hello");
        pt.insert(4, " ");
        let mut pieces = PieceIter::new(&pt.pt, 0);

        assert_eq!(add_piece(0, 0, 4), pieces.get());
        assert_eq!(add_piece(4, 5, 1), pieces.next());
        assert_eq!(add_piece(5, 4, 1), pieces.next());
    }

    #[test]
    fn bounded1() {
        let mut pt = PieceTree::new();
        pt.insert(0, "baz"); // pos 6, buf 0
        pt.insert(0, "bar"); // pos 3, buf 3
        pt.insert(0, "foo"); // pos 0, buf 6
        let mut pieces = BoundedPieceIter::new_from_slice(&pt.slice(2..7), 0); // fo(obarb)az

        assert_eq!(add_piece(0, 8, 1), pieces.get()); // o
        assert_eq!(add_piece(1, 3, 3), pieces.next()); // bar
        assert_eq!(add_piece(4, 0, 1), pieces.next()); // b
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(4, 0, 1), pieces.prev());
        assert_eq!(add_piece(1, 3, 3), pieces.prev());
        assert_eq!(add_piece(0, 8, 1), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 8, 1), pieces.get());
    }

    #[test]
    fn bounded2() {
        let mut pt = PieceTree::new();
        pt.insert(0, "baz");
        pt.insert(0, "bar");
        pt.insert(0, "foo");
        let mut pieces = BoundedPieceIter::new_from_slice(&pt.slice(3..6), 0);

        assert_eq!(add_piece(0, 3, 3), pieces.get());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(0, 3, 3), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 3, 3), pieces.get());
    }

    #[test]
    fn bounded3() {
        let mut pt = PieceTree::new();
        pt.insert(0, "baz"); // pos 6, buf 0
        pt.insert(0, "bar"); // pos 3, buf 3
        pt.insert(0, "foo"); // pos 0, buf 6
        let mut pieces = BoundedPieceIter::new_from_slice(&pt.slice(..), 0);

        assert_eq!(add_piece(0, 6, 3), pieces.get()); // foo
        assert_eq!(add_piece(3, 3, 3), pieces.next()); // bar
        assert_eq!(add_piece(6, 0, 3), pieces.next()); // baz
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(6, 0, 3), pieces.prev());
        assert_eq!(add_piece(3, 3, 3), pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.get());
    }
}
