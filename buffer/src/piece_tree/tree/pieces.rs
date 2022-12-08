use std::ops::Range;

use super::node::internal_node::InternalNode;
use super::node::Node;
use super::piece::Piece;
use crate::cursor_iterator::CursorIterator;
use crate::piece_tree::PieceTree;

/// Traverse pieces in the tree, in order
#[derive(Debug, Clone)]
pub(crate) struct Pieces<'a> {
    pt: &'a PieceTree,
    stack: Vec<&'a InternalNode>,
    pos: usize,          // Current piece pos in buffer
    range: Range<usize>, // Limit to only a part of the buffer
}

impl<'a> Pieces<'a> {
    #[inline]
    pub(crate) fn new(pt: &'a PieceTree, at: usize, range: Range<usize>) -> Self {
        let at = range.start + at;
        // Be empty at pt.len
        let (stack, pos) = if at == pt.len {
            (Vec::with_capacity(pt.tree.max_height()), at)
        } else {
            pt.tree.find_node(at)
        };
        Pieces {
            pt,
            stack,
            pos,
            range,
        }
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

    #[inline]
    fn next_piece(&mut self) -> Option<()> {
        let prev_len = (*self.stack.last()?).piece.len;
        self.tree_next()?;
        self.pos += prev_len;
        Some(())
    }

    #[inline]
    fn prev_piece(&mut self) -> Option<()> {
        let p = self.tree_prev()?;
        self.pos -= p.len;
        Some(())
    }
}

impl<'a> CursorIterator for Pieces<'a> {
    type Item = (usize, Piece);

    #[inline(always)]
    fn get(&self) -> Option<(usize, Piece)> {
        let mut piece: Piece = (*self.stack.last()?).piece.clone();
        let mut p_start = self.pos;
        let p_end = p_start + piece.len;

        let Range { start, end } = self.range;

        // Shrink the piece if bounds are met
        if p_start < start {
            let diff = start - p_start;
            piece.split_left(diff);
            p_start += diff;
        }

        if end < p_end {
            piece.split_right(p_end - end);
        }

        if piece.len == 0 {
            return None;
        }

        Some((p_start, piece))
    }

    #[inline]
    fn next(&mut self) -> Option<(usize, Piece)> {
        let over_bounds = self
            .stack
            .last()
            .map(|n| {
                let piece = &n.piece;
                let p_start = self.pos;
                let p_end = p_start + piece.len;
                let Range { end, .. } = self.range;
                end < p_end
            })
            .unwrap_or(false);

        if over_bounds {
            return None;
        }

        self.next_piece()?;
        self.get()

        // if piece_is_over_bounds(piece, self.index, self.min_bound, self.max_bound) {
        //     return;
        // }
        // }

        // self.cur_piece = self.next_piece();

        // let prev_len = self.get()?.1.len;

        // if let Some(p) = self.tree_next().cloned() {
        //     self.pos += prev_len;
        //     Some((self.pos, p))
        // } else {
        //     self.pos = self.pt.len;
        //     None
        // }
    }

    #[inline]
    fn prev(&mut self) -> Option<(usize, Piece)> {
        // Restart iteration from last position if we went past it
        if self.stack.len() == 0 {
            let (stack, index) = self.pt.tree.find_node(self.range.end);
            self.stack = stack;
            self.pos = index;
            return self.get();
        }

        // Don't iterate and empty stack if we are at the first piece
        if self.pos == 0 {
            return None;
        }

        // Don't iterate if this piece is already over the bounds
        let over_bounds = self
            .stack
            .last()
            .map(|_| {
                let p_start = self.pos;
                let Range { start, .. } = self.range;
                p_start < start
            })
            .unwrap_or(false);

        if over_bounds {
            return None;
        }

        self.prev_piece()?;
        self.get()
    }

    #[inline(always)]
    fn pos(&self) -> usize {
        self.pos
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
        let pieces = Pieces::new(&pt, 0, 0..pt.len());
        assert_eq!(None, pieces.get());
    }

    #[test]
    fn piece_one() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "foobar");
        let mut pieces = Pieces::new(&pt, 0, 0..pt.len());

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
        let mut pieces = Pieces::new(&pt, 0, 0..pt.len());

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
        let mut pieces = Pieces::new(&pt, 5, 0..pt.len());

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
        let pieces = Pieces::new(&pt, pt.len, 0..pt.len());

        assert_eq!(None, pieces.get());
    }

    #[test]
    fn length_1() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "hello");
        pt.insert_str(4, " ");
        let mut pieces = Pieces::new(&pt, 0, 0..pt.len());

        assert_eq!(add_piece(0, 0, 4), pieces.get());
        assert_eq!(add_piece(4, 5, 1), pieces.next());
        assert_eq!(add_piece(5, 4, 1), pieces.next());
    }

    #[test]
    fn slice_3() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "baz");
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut pieces = Pieces::new(&pt, 2, 0..3);

        assert_eq!(add_piece(0, 6, 3), pieces.get());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(0, 6, 3), pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.get());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.get());
    }

    #[test]
    fn slice_5_middle() {
        let mut pt = PieceTree::new();
        pt.insert_str(0, "baz");
        pt.insert_str(0, "bar");
        pt.insert_str(0, "foo");
        let mut pieces = Pieces::new(&pt, 2, 2..7);

        assert_eq!(add_piece(0, 6, 3), pieces.get());
        assert_eq!(None, pieces.next());
        assert_eq!(None, pieces.get());
        assert_eq!(add_piece(0, 6, 3), pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.get());
        assert_eq!(None, pieces.prev());
        assert_eq!(add_piece(0, 6, 3), pieces.get());
    }
}
