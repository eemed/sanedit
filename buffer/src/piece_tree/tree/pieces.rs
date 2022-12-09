use std::ops::Range;

use super::node::internal_node::InternalNode;
use super::node::Node;
use super::piece::Piece;
use crate::cursor_iterator::CursorIterator;
use crate::piece_tree::PieceTree;

/// Traverse pieces in the tree, in order
#[derive(Debug, Clone)]
pub(crate) struct PieceIter<'a> {
    pt: &'a PieceTree,
    stack: Vec<&'a InternalNode>,
    pos: usize, // Current piece pos in buffer
}

impl<'a> PieceIter<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTree, at: usize) -> Self {
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
}

impl<'a> CursorIterator for PieceIter<'a> {
    type Item = (usize, Piece);

    #[inline(always)]
    fn get(&self) -> Option<(usize, Piece)> {
        let piece = self.stack.last().map(|&node| node.piece.clone())?;
        let pos = self.pos();
        Some((pos, piece))
    }

    #[inline]
    fn next(&mut self) -> Option<(usize, Piece)> {
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
    fn prev(&mut self) -> Option<(usize, Piece)> {
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
    fn pos(&self) -> usize {
        self.pos
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BoundedPieceIter<'a> {
    range: Range<usize>,
    iter: PieceIter<'a>,
}

impl<'a> BoundedPieceIter<'a> {
    pub fn new(pt: &'a PieceTree, at: usize, range: Range<usize>) -> BoundedPieceIter<'a> {
        let iter = PieceIter::new(pt, range.start + at);
        BoundedPieceIter { range, iter }
    }
}

impl<'a> CursorIterator for BoundedPieceIter<'a> {
    type Item = (usize, Piece);

    fn pos(&self) -> usize {
        // TODO remove this
        0
    }

    fn get(&self) -> Option<Self::Item> {
        let (mut p_start, mut piece) = self.iter.get()?;
        let p_end = p_start + piece.len;

        let Range { start, end } = self.range;

        // Shrink the piece if bounds are met
        if p_start < start {
            let diff = start - p_start;
            piece.split_right(diff);
            p_start += diff;
        }

        if end < p_end {
            piece.split_left(piece.len - (p_end - end));
        }

        if piece.len == 0 {
            return None;
        }

        Some((p_start, piece))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let (p_start, _) = self.iter.get()?;
        let Range { end, .. } = self.range;
        let over_bounds = end < p_start;

        if over_bounds {
            return None;
        }

        self.iter.next()?;
        self.get()
    }

    fn prev(&mut self) -> Option<Self::Item> {
        if let Some((p_start, _)) = self.iter.get() {
            let Range { start, .. } = self.range;
            let over_bounds = p_start <= start;

            if over_bounds {
                return None;
            }
        }

        self.iter.prev()?;
        self.get()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::piece_tree::buffers::BufferKind;

    use super::*;

    fn add_piece(pos: usize, index: usize, len: usize) -> Option<(usize, Piece)> {
        Some((pos, Piece::new(BufferKind::Add, index, len)))
    }

    #[test]
    fn empty() {
        let pt = PieceTree::new();
        let pieces = PieceIter::new(&pt, 0);
        assert_eq!(None, pieces.get());
        assert_eq!(0, pieces.pos());
    }

    #[test]
    fn piece_one() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foobar");
        let mut pieces = PieceIter::new(&pt, 0);

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
        pt.insert_str(0, "baz");
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut pieces = PieceIter::new(&pt, 0);

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
        pt.insert_str(0, "baz");
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut pieces = PieceIter::new(&pt, 5);

        assert_eq!(add_piece(3, 3, 3), pieces.get());
        assert_eq!(add_piece(6, 0, 3), pieces.next());
        assert_eq!(None, pieces.next());
    }

    #[test]
    fn at_max() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "baz");
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let pieces = PieceIter::new(&pt, pt.len);

        assert_eq!(None, pieces.get());
    }

    #[test]
    fn length_1() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "hello");
        pt.insert_str(4, " ");
        let mut pieces = PieceIter::new(&pt, 0);

        assert_eq!(add_piece(0, 0, 4), pieces.get());
        assert_eq!(add_piece(4, 5, 1), pieces.next());
        assert_eq!(add_piece(5, 4, 1), pieces.next());
    }

    #[test]
    fn bounded1() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "baz"); // pos 6, buf 0
        pt.insert_str(0, "bar"); // pos 3, buf 3
        pt.insert_str(0, "foo"); // pos 0, buf 6
        let mut pieces = BoundedPieceIter::new(&pt, 0, 2..7); // fo(obarb)az

        assert_eq!(add_piece(2, 8, 1), pieces.get()); // o
        assert_eq!(add_piece(3, 3, 3), pieces.next()); // bar
        assert_eq!(add_piece(6, 0, 1), pieces.next()); // b
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(6, 0, 1), pieces.prev());
        assert_eq!(add_piece(3, 3, 3), pieces.prev());
        assert_eq!(add_piece(2, 8, 1), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(2, 8, 1), pieces.get());
    }

    #[test]
    fn bounded2() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "baz");
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut pieces = BoundedPieceIter::new(&pt, 0, 3..6);

        assert_eq!(add_piece(3, 3, 3), pieces.get());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(3, 3, 3), pieces.prev());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(3, 3, 3), pieces.get());
    }
}
