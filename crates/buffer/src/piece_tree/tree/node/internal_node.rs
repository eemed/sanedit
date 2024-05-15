use std::mem;
use std::sync::Arc;

use crate::piece_tree::tree::color::Color;
use crate::piece_tree::tree::piece::Piece;

use super::Node;

/// Internal node in the red black tree.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InternalNode {
    pub(crate) left: Arc<Node>,
    pub(crate) right: Arc<Node>,
    pub(crate) color: Color,

    /// Data in the tree
    pub(crate) piece: Piece,
    /// Left subtree length in bytes
    pub(crate) left_subtree_len: usize,
}

impl InternalNode {
    pub fn new(color: Color, piece: Piece) -> InternalNode {
        InternalNode {
            left: Arc::new(Node::Leaf),
            right: Arc::new(Node::Leaf),
            color,
            piece,
            left_subtree_len: 0,
        }
    }

    /// Matt Might's deletion function. Bubbles up the BlackBlack nodes.
    pub fn bubble(&mut self) {
        use Color::BlackBlack as BB;

        if self.left.color() == BB || self.right.color() == BB {
            self.blacken();

            let left = Arc::make_mut(&mut self.left);
            left.redden();

            let right = Arc::make_mut(&mut self.right);
            right.redden();
        }

        self.balance();
    }

    #[inline]
    pub fn blacken(&mut self) {
        self.color.blacken();
    }

    #[inline]
    pub fn redden(&mut self) {
        self.color.redden();
    }

    #[inline]
    pub fn take_left(&mut self) -> Arc<Node> {
        mem::replace(&mut self.left, Arc::new(Node::Leaf))
    }

    #[inline]
    pub fn take_right(&mut self) -> Arc<Node> {
        mem::replace(&mut self.right, Arc::new(Node::Leaf))
    }

    pub fn insert_left(&mut self, piece: Piece) {
        fn ins_right(node: &mut InternalNode, piece: Piece) {
            let right = Arc::make_mut(&mut node.right);
            match right {
                Node::Internal(r) => {
                    ins_right(r, piece);
                    r.balance();
                }
                _ => {
                    node.right = Arc::new(InternalNode::new(Color::Red, piece).into());
                }
            }
        }

        let left = Arc::make_mut(&mut self.left);

        match left {
            Node::Internal(l) => {
                ins_right(l, piece);
                l.balance();
            }
            _ => {
                self.left = Arc::new(InternalNode::new(Color::Red, piece).into());
            }
        }
    }

    pub fn insert_right(&mut self, piece: Piece) {
        fn ins_left(node: &mut InternalNode, piece: Piece) {
            node.left_subtree_len += piece.len;

            let left = Arc::make_mut(&mut node.left);
            match left {
                Node::Internal(l) => {
                    ins_left(l, piece);
                    l.balance();
                }
                _ => {
                    node.left = Arc::new(InternalNode::new(Color::Red, piece).into());
                }
            }
        }

        let right = Arc::make_mut(&mut self.right);
        match right {
            Node::Internal(r) => {
                ins_left(r, piece);
                r.balance();
            }
            _ => {
                self.right = Arc::new(InternalNode::new(Color::Red, piece).into());
            }
        }
    }

    /// Balance function using Chris Okasakis insertion method and Matt Mights
    /// deletion method.
    pub fn balance(&mut self) {
        use Color::{Black as B, NegativeBlack as NB, Red as R};

        #[inline]
        fn internal_color(n: &Arc<Node>) -> Option<Color> {
            match n.as_ref() {
                Node::Internal(n) => Some(n.color),
                _ => None,
            }
        }

        #[inline]
        fn internal_tree_colors(n: &Arc<Node>) -> (Option<Color>, Option<Color>, Option<Color>) {
            match n.as_ref() {
                Node::Internal(n) => {
                    let left = internal_color(&n.left);
                    let right = internal_color(&n.right);
                    (Some(n.color), left, right)
                }
                _ => (None, None, None),
            }
        }

        if self.color == R || self.color == NB {
            return;
        }

        // Color is Black or BlackBlack

        let (color_l, color_l_l, color_l_r) = internal_tree_colors(&self.left);
        let (color_r, color_r_l, color_r_r) = internal_tree_colors(&self.right);

        match (color_l, color_l_l, color_l_r, color_r, color_r_l, color_r_r) {
            // Okasakis insertion cases and Mights deletion combined as they
            // are very similar
            (Some(R), Some(R), ..) => {
                //       zB                     yR
                //      / \                    /  \
                //     yR   d                 xB   zB
                //    / \          ==>       / \   / \
                //   xR   c                 a   b c   d
                //  / \
                // a   b
                let mut y_ptr = self.take_left();
                let y = Arc::make_mut(&mut y_ptr).internal();
                let mut x_ptr = y.take_left();
                let x = Arc::make_mut(&mut x_ptr).internal();

                self.color.redden();
                y.color = B;
                x.color = B;

                self.left_subtree_len -= y.piece.len + y.left_subtree_len;

                mem::swap(&mut self.piece, &mut y.piece);
                mem::swap(&mut self.left_subtree_len, &mut y.left_subtree_len);
                mem::swap(&mut y.left, &mut y.right);
                mem::swap(&mut self.right, &mut y.right);

                self.left = x_ptr;
                self.right = y_ptr;
            }
            (Some(R), _, Some(R), ..) => {
                //       zB                     yR
                //      / \                    /  \
                //     xR   d                 xB   zB
                //    / \          ==>       / \   / \
                //   a   yR                 a   b c   d
                //      / \
                //     b   c
                let mut x_ptr = self.take_left();
                let x = Arc::make_mut(&mut x_ptr).internal();
                let mut y_ptr = x.take_right();
                let y = Arc::make_mut(&mut y_ptr).internal();

                self.redden();
                x.color = B;
                y.color = B;

                self.left_subtree_len -=
                    x.piece.len + x.left_subtree_len + y.left_subtree_len + y.piece.len;

                y.left_subtree_len += x.piece.len + x.left_subtree_len;

                mem::swap(&mut self.piece, &mut y.piece);
                mem::swap(&mut self.left_subtree_len, &mut y.left_subtree_len);
                mem::swap(&mut y.left, &mut y.right);
                mem::swap(&mut x.right, &mut y.right);
                mem::swap(&mut self.right, &mut y.right);

                self.right = y_ptr;
                self.left = x_ptr;
            }
            (.., Some(R), Some(R), _) => {
                //       xB                     yR
                //      / \                    /  \
                //     a   zR                 xB   zB
                //        /  \     ==>       / \   / \
                //       yR   d             a   b c   d
                //      / \
                //     b   c
                let mut z_ptr = self.take_right();
                let z = Arc::make_mut(&mut z_ptr).internal();
                let mut y_ptr = z.take_left();
                let y = Arc::make_mut(&mut y_ptr).internal();

                self.color.redden();
                z.color = B;
                y.color = B;

                z.left_subtree_len -= y.left_subtree_len + y.piece.len;
                y.left_subtree_len += self.left_subtree_len + self.piece.len;

                mem::swap(&mut self.piece, &mut y.piece);
                mem::swap(&mut self.left_subtree_len, &mut y.left_subtree_len);
                mem::swap(&mut z.left, &mut y.right);
                mem::swap(&mut y.left, &mut y.right);
                mem::swap(&mut self.left, &mut y.left);

                self.left = y_ptr;
                self.right = z_ptr;
            }
            (.., Some(R), _, Some(R)) => {
                //       xB                     yR
                //      / \                    /  \
                //     a   yR                 xB   zB
                //        /  \     ==>       / \   / \
                //       b    zR            a   b c   d
                //           /  \
                //          c    d
                let mut y_ptr = self.take_right();
                let y = Arc::make_mut(&mut y_ptr).internal();
                let mut z_ptr = y.take_right();
                let z = Arc::make_mut(&mut z_ptr).internal();

                self.color.redden();
                y.color = B;
                z.color = B;

                y.left_subtree_len += self.left_subtree_len + self.piece.len;

                mem::swap(&mut self.piece, &mut y.piece);
                mem::swap(&mut self.left_subtree_len, &mut y.left_subtree_len);
                mem::swap(&mut y.left, &mut y.right);
                mem::swap(&mut self.left, &mut y.left);

                self.right = z_ptr;
                self.left = y_ptr;
            }
            // Mights negative black cases
            (.., Some(NB), Some(B), Some(B)) => {
                //        xBB                    yB
                //       / \                    /  \
                //      a   zNB                xB   zB
                //         /   \     ==>      / \   / \
                //        yB    wB           a   b c   wR
                //       / \   /  \                   /  \
                //      b   c d    e                 d    e
                //
                let mut z_ptr = self.take_right();
                let z = Arc::make_mut(&mut z_ptr).internal();
                let mut y_ptr = z.take_left();
                let y = Arc::make_mut(&mut y_ptr).internal();
                let mut w_ptr = z.take_right();
                let w = Arc::make_mut(&mut w_ptr).internal();

                self.color = B;
                z.color = B;
                y.color = B;
                w.color = R;

                z.left_subtree_len -= y.piece.len + y.left_subtree_len;
                y.left_subtree_len = self.left_subtree_len + y.left_subtree_len + self.piece.len;

                mem::swap(&mut self.piece, &mut y.piece);
                mem::swap(&mut self.left_subtree_len, &mut y.left_subtree_len);
                mem::swap(&mut y.left, &mut y.right);
                mem::swap(&mut y.left, &mut self.left);

                mem::swap(&mut self.left, &mut z.left);

                z.right = w_ptr;
                z.balance();

                self.left = y_ptr;
                self.right = z_ptr;
            }
            (Some(NB), Some(B), Some(B), ..) => {
                //        zBB                    yB
                //       /   \                  /  \
                //      xNB   d                xB   zB
                //    /    \         ==>      / \   / \
                //   wB    yB                wR  b c   d
                //  / \    / \              /  \
                // a'  b' b   c            a'   b'
                //
                let mut x_ptr = self.take_left();
                let x = Arc::make_mut(&mut x_ptr).internal();
                let mut w_ptr = x.take_left();
                let w = Arc::make_mut(&mut w_ptr).internal();
                let mut y_ptr = x.take_right();
                let y = Arc::make_mut(&mut y_ptr).internal();

                self.color = B;
                x.color = B;
                y.color = B;
                w.color = R;

                self.left_subtree_len -=
                    x.piece.len + x.left_subtree_len + y.piece.len + y.left_subtree_len;

                y.left_subtree_len += x.piece.len + x.left_subtree_len;

                mem::swap(&mut self.piece, &mut y.piece);
                mem::swap(&mut self.left_subtree_len, &mut y.left_subtree_len);
                mem::swap(&mut y.left, &mut y.right);
                mem::swap(&mut self.right, &mut y.right);
                mem::swap(&mut self.right, &mut x.right);

                x.left = w_ptr;
                x.balance();

                self.left = x_ptr;
                self.right = y_ptr;
            }
            _ => {}
        }
    }

    // fn print_as_tree(&self) {
    //     fn print(node: &InternalNode, mut space: usize) {
    //         space += 10;
    //         if let Node::Internal(right) = node.right.as_ref() {
    //             print(right, space);
    //         }
    //         // println!();
    //         print!("{}", " ".repeat(space - 10));
    //         println!(
    //             "{:?}, {:?}, {:?}",
    //             node.color, node.left_subtree_len, node.piece
    //             );
    //         if let Node::Internal(left) = node.left.as_ref() {
    //             print(left, space);
    //         }
    //     }

    //     print(self, 0);
    // }
}
