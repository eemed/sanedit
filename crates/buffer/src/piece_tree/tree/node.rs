pub(crate) mod internal_node;

use std::mem;
use std::sync::Arc;

use self::internal_node::InternalNode;
use super::color::Color;
use super::piece::Piece;

/// Red black tree node types
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Leaf,
    BBLeaf,
    Internal(InternalNode),
}

impl Node {
    #[inline]
    pub fn new(color: Color, piece: Piece) -> Node {
        Node::Internal(InternalNode::new(color, piece))
    }

    #[inline]
    pub fn color(&self) -> Color {
        match self {
            Node::Internal(n) => n.color,
            Node::Leaf => Color::Black,
            Node::BBLeaf => Color::BlackBlack,
        }
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        !matches!(self, Node::Internal(_))
    }

    #[inline]
    pub fn redden(&mut self) {
        match self {
            Node::Leaf => unreachable!(),
            Node::BBLeaf => {
                *self = Node::Leaf;
            }
            Node::Internal(n) => {
                n.redden();
            }
        }
    }

    #[inline]
    pub fn internal(&mut self) -> Option<&mut InternalNode> {
        match self {
            Node::Internal(n) => Some(n),
            _ => None,
        }
    }

    #[inline]
    pub fn internal_ref(&self) -> Option<&InternalNode> {
        match self {
            Node::Internal(n) => Some(n),
            _ => None,
        }
    }

    pub fn remove(&mut self) {
        match self {
            Node::Internal(n) => match (n.left.is_leaf(), n.right.is_leaf()) {
                (true, true) => match self.color() {
                    Color::Red => {
                        *self = Node::Leaf;
                    }
                    Color::Black => {
                        *self = Node::BBLeaf;
                    }
                    _ => unreachable!(),
                },
                (true, false) => {
                    if n.color == Color::Black && n.right.color() == Color::Red {
                        let mut right = n.take_right();
                        let right = Arc::make_mut(&mut right).internal().unwrap();
                        mem::swap(n, right);
                        n.color = Color::Black;
                    }
                }
                (false, true) => {
                    if n.color == Color::Black && n.left.color() == Color::Red {
                        let mut left = n.take_left();
                        let left = Arc::make_mut(&mut left).internal().unwrap();
                        mem::swap(n, left);
                        n.color = Color::Black;
                    }
                }
                (false, false) => {
                    let left = Arc::make_mut(&mut n.left);
                    let piece = left.remove_max();
                    n.left_subtree_len -= piece.len;
                    n.piece = piece;
                    n.bubble();
                }
            },
            _ => unreachable!(),
        }
    }

    pub fn remove_max(&mut self) -> Piece {
        fn rec(node: &mut Node) -> Piece {
            match node {
                Node::Internal(n) => {
                    if n.right.is_leaf() {
                        // Remove this node
                        let piece = n.piece.clone();
                        node.remove();
                        piece
                    } else {
                        // Recurse into child
                        let right = Arc::make_mut(&mut n.right);
                        let piece = rec(right);
                        n.bubble();
                        piece
                    }
                }
                _ => unreachable!(),
            }
        }

        rec(self)
    }
}

impl From<InternalNode> for Node {
    fn from(n: InternalNode) -> Self {
        Node::Internal(n)
    }
}
