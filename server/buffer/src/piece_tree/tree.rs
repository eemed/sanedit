pub(crate) mod color;
pub(crate) mod node;
pub(crate) mod piece;
pub(crate) mod pieces;

use std::ops::Range;
use std::rc::Rc;

use self::color::Color;
use self::node::internal_node::InternalNode;
use self::node::Node;
use self::piece::Piece;

use super::buffers::BufferKind;

#[derive(Clone, Debug)]
pub(crate) struct Tree {
    pub(crate) root: Rc<Node>,
    pub(crate) node_count: usize,
}

impl Tree {
    #[inline]
    pub fn new() -> Tree {
        Tree {
            root: Rc::new(Node::Leaf),
            node_count: 0,
        }
    }

    #[inline(always)]
    pub fn max_height(&self) -> usize {
        2 * (self.node_count as f64).log2() as usize
    }

    /// Insert piece `piece` to tree at index `index`.
    #[inline]
    pub fn insert(&mut self, pos: usize, piece: Piece) {
        let (nodes_inserted, ..) = insert_rec(&mut self.root, pos, piece, true);
        self.node_count += nodes_inserted;
    }

    pub fn remove(&mut self, range: Range<usize>) {
        let mut removed_bytes = 0;
        let len = range.len();

        while removed_bytes < len {
            let (removed_piece, node_removed, ins_p) =
                remove_rec(&mut self.root, range.start, len - removed_bytes, true);

            if node_removed {
                self.node_count -= 1;
            }

            removed_bytes += removed_piece.len;

            if let Some(p) = ins_p {
                removed_bytes -= p.len;

                let (nodes_inserted, _) = insert_rec(&mut self.root, range.start, p, true);
                self.node_count += nodes_inserted;
            }
        }
    }

    pub fn find_node(&self, mut target: usize) -> (Vec<&InternalNode>, usize) {
        let mut pos = 0;
        let mut stack = Vec::with_capacity(self.max_height());
        let mut node = self.root.as_ref();

        if node.is_leaf() {
            return (stack, pos);
        }

        loop {
            let n = node.internal_ref().unwrap();

            let node_left_len = n.left_subtree_len;
            let node_piece = &n.piece;

            pos += node_left_len;

            if node_left_len > target {
                stack.push(n);
                pos -= node_left_len;
                node = &n.left;
            } else if node_left_len == target
                || node_left_len + node_piece.len > target
                || node_left_len + node_piece.len == target && n.right.is_leaf()
            {
                stack.push(n);
                return (stack, pos);
            } else {
                stack.push(n);
                target -= node_left_len + node_piece.len;
                pos += node_piece.len;
                node = &n.right;
            }
        }
    }
}

/// Returns:
///     number of inserted nodes
///     inserted byte count
///     inserted line count
fn insert_rec(
    node: &mut Rc<Node>,
    mut index: usize, // Index in buffer
    piece: Piece,     // Piece to insert
    at_root: bool,
) -> (usize, usize) {
    if node.is_leaf() {
        let ins_bytes = piece.len;
        let node_color = if at_root { Color::Black } else { Color::Red };
        *node = Rc::new(Node::new(node_color, piece));

        return (1, ins_bytes);
    }

    let node = Rc::make_mut(node).internal().unwrap();
    let node_left_len = node.left_subtree_len;
    let node_piece = &node.piece;

    let (nodes_added, ins_bytes) = if node_left_len > index {
        let ret = insert_rec(&mut node.left, index, piece, false);

        node.left_subtree_len += ret.1;
        ret
    } else if node_left_len == index {
        let ins_bytes = piece.len;
        node.insert_left(piece);

        node.left_subtree_len += ins_bytes;
        (1, ins_bytes)
    } else if node_left_len + node_piece.len == index {
        // Append?
        if node_piece.kind == BufferKind::Add && node_piece.pos + node_piece.len == piece.pos {
            node.piece.len += piece.len;
            (0, piece.len)
        } else {
            // Otherwise insert to the right side
            let ins_bytes = piece.len;
            node.insert_right(piece);
            (1, ins_bytes)
        }
    } else if node_left_len + node_piece.len > index {
        // Index is in the middle of the piece split the current piece.
        let right_piece = node.piece.split_left(index - node_left_len);

        // Remove right piece length from left subtree lengths because we are
        // temporarily removing it.
        let ins_bytes = piece.len;
        node.insert_right(right_piece);
        node.insert_right(piece);

        (2, ins_bytes)
    } else {
        // node_left_len + node_piece_len < index
        // Go right
        index -= node_left_len + node_piece.len;
        insert_rec(&mut node.right, index, piece, false)
    };

    if nodes_added > 0 {
        node.balance();
    }

    if at_root {
        node.color = Color::Black;
    }

    (nodes_added, ins_bytes)
}

/// Remove from `len` bytes at position `index`.
/// Returns:
///     Removed piece,
///     Wether the whole node was removed.
///     Optional piece to insert if a piece split was needed.
fn remove_rec(
    node: &mut Rc<Node>,
    mut index: usize, // Remove buffer position
    len: usize,       // Remove length
    at_root: bool,
) -> (Piece, bool, Option<Piece>) {
    if node.is_leaf() {
        unreachable!("Remove rec found leaf node");
    }

    // Get to the internal node
    let node_ref = Rc::make_mut(node);
    let n = node_ref.internal().unwrap();
    let n_left_len = n.left_subtree_len;
    let n_piece_len = n.piece.len;

    let (rem_p, node_removed, ins_p, remove_piece) = if n_left_len > index {
        let (removed_piece, node_removed, ins_p) = remove_rec(&mut n.left, index, len, false);
        n.left_subtree_len -= removed_piece.len;
        (removed_piece, node_removed, ins_p, false)
    } else if n_left_len == index {
        if len >= n_piece_len {
            // Remove whole piece
            (n.piece.clone(), true, None, true)
        } else {
            let rem_p = n.piece.split_right(len);
            (rem_p, false, None, false)
        }
    } else if n_left_len + n_piece_len > index {
        // Removing from middle
        let mut right_p = n.piece.split_left(index - n_left_len);

        let rem_p = right_p.clone();
        let ins_p = if len >= right_p.len {
            // Whole right piece is removed.
            None
        } else {
            // A part of right piece is removed.
            // We need to reinsert the remaining part
            right_p.split_right(len);
            Some(right_p)
        };

        (rem_p, false, ins_p, false)
    } else {
        index -= n_left_len + n_piece_len;
        let (removed_piece, node_removed, ins_p) = remove_rec(&mut n.right, index, len, false);
        (removed_piece, node_removed, ins_p, false)
    };

    if remove_piece {
        node_ref.remove();
    } else if node_removed {
        n.bubble();
    }

    if at_root {
        if let Node::Internal(n) = node_ref {
            n.color = Color::Black;
        } else {
            *node = Rc::new(Node::Leaf);
        }
    }

    (rem_p, node_removed, ins_p)
}

/// Find a node at `index` in the tree.
#[inline]
pub(crate) fn find_node_at(tree: &Tree, pos: usize) -> (Vec<&InternalNode>, usize) {
    if tree.root.is_leaf() {
        return (vec![], 0);
    }

    let mut stack = Vec::with_capacity(tree.max_height());
    let idx = find_node_rec(&tree.root, pos, 0, &mut stack);
    (stack, idx)
}

fn find_node_rec<'a>(
    node: &'a Node,
    mut pos: usize,
    mut cur_pos: usize,
    stack: &mut Vec<&'a InternalNode>,
) -> usize {
    if let Node::Internal(n) = node {
        let node_left_len = n.left_subtree_len;
        let node_piece = &n.piece;

        cur_pos += node_left_len;

        if node_left_len > pos {
            stack.push(n);
            return find_node_rec(&n.left, pos, cur_pos - node_left_len, stack);
        } else if node_left_len == pos
            || node_left_len + node_piece.len > pos
            || node_left_len + node_piece.len == pos && n.right.is_leaf()
        {
            stack.push(n);
            return cur_pos;
        } else {
            stack.push(n);
            pos -= node_left_len + node_piece.len;
            cur_pos += node_piece.len;
            return find_node_rec(&n.right, pos, cur_pos, stack);
        }
    }

    unreachable!()
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use rand_chacha::rand_core::{RngCore, SeedableRng};

    use super::*;
    use crate::piece_tree::{
        buffers::{AddBuffer, OriginalBuffer},
        PieceTree,
    };

    #[test]
    fn find_node_start() {
        let pt = complex_tree();
        let (stack, pos) = pt.tree.find_node(0);

        assert_eq!(0, pos);
        assert_eq!(3, stack.len());
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 16,
                len: 2
            },
            stack[0].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 5,
                len: 2
            },
            stack[1].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 0,
                len: 2
            },
            stack[2].piece
        );
    }

    #[test]
    fn find_node_middle() {
        let pt = complex_tree();
        let (stack, pos) = pt.tree.find_node(pt.len / 2);

        assert_eq!(9, pos);
        assert_eq!(4, stack.len());
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 16,
                len: 2
            },
            stack[0].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 12,
                len: 2
            },
            stack[1].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 18,
                len: 2
            },
            stack[2].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 20,
                len: 2
            },
            stack[3].piece
        );
    }

    #[test]
    fn find_node_end() {
        let pt = complex_tree();
        let (stack, pos) = pt.tree.find_node(pt.len);

        assert_eq!(17, pos);
        assert_eq!(3, stack.len());
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 16,
                len: 2
            },
            stack[0].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 12,
                len: 2
            },
            stack[1].piece
        );
        assert_eq!(
            Piece {
                kind: BufferKind::Add,
                pos: 10,
                len: 2
            },
            stack[2].piece
        );
    }

    #[test]
    fn insert_at_start() {
        let mut pt = PieceTree::new();

        pt.insert(0, "abcde");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        pt.insert(0, "ab");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn insert_at_middle() {
        let mut pt = PieceTree::new();

        pt.insert(0, "abcde");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        pt.insert(2, "ab");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn insert_at_end() {
        let mut pt = PieceTree::new();

        pt.insert(0, "abcde");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        pt.insert(5, "ab");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_left_child() {
        let mut pt = simple_tree();
        pt.remove(0..1);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_right_child() {
        let mut pt = simple_tree();
        pt.remove(2..3);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_root() {
        let mut pt = simple_tree();
        pt.remove(1..2);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_start() {
        let mut pt = one_piece_tree();
        pt.remove(0..5);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_middle() {
        let mut pt = one_piece_tree();
        pt.remove(2..7);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_end() {
        let mut pt = one_piece_tree();
        pt.remove(5..);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_over_whole_piece() {
        let mut pt = PieceTree::new();
        pt.insert(0, "ab");
        pt.add.extend_from_slice(b"123");
        pt.insert(2, "cd");
        pt.add.extend_from_slice(b"123");
        pt.insert(4, "ef");

        pt.remove(1..4);
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_ll() {
        let mut pt = complex_tree();

        pt.remove(0..1);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());

        pt.remove(0..1);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());
    }

    #[test]
    fn remove_lr() {
        let mut pt = complex_tree();

        pt.remove(4..5);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());

        pt.remove(4..5);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());
    }

    #[test]
    fn remove_rl() {
        let mut pt = complex_tree();

        pt.remove(1..2);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());

        pt.remove(1..2);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());
    }

    #[test]
    fn remove_rr() {
        let mut pt = complex_tree();

        pt.remove(1..8);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());

        pt.remove(1..8);

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());
    }

    #[test]
    fn remove_complex_middle() {
        let mut pt = complex_tree();

        for _ in 0..pt.len() {
            let pos = pt.len() / 2;
            pt.remove(pos..pos + 1);

            assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        }
    }

    #[test]
    fn remove_complex_start() {
        let mut pt = complex_tree();

        for _ in 0..pt.len() {
            let pos = 0;
            pt.remove(pos..pos + 1);

            assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        }
    }

    #[test]
    fn remove_complex_end() {
        let mut pt = complex_tree();

        for _ in 0..pt.len() {
            let pos = pt.len().saturating_sub(2);
            pt.remove(pos..pos + 1);

            assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        }
    }

    #[test]
    fn remove_complex_whole() {
        let mut pt = complex_tree();
        pt.remove(0..pt.len());
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
    }

    #[test]
    fn remove_complex_end_medium() {
        let mut pt = complex_tree();

        let pos = pt.len() / 2;
        let end = (pos + 15).min(pt.len());
        for _ in pos..end {
            let pos = pt.len().saturating_sub(2);
            pt.remove(pos..pos + 1);

            assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        }
    }

    #[test]
    fn fuzz_found_bug_1() {
        fn make_tree(p_len: usize) -> PieceTree {
            let mut pt = PieceTree::new();
            pt.insert(0, "a".repeat(p_len).as_bytes());
            pt
        }

        let seed = 67_319;
        let mut gen = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        let p_len = 1000;
        let mut tree_len = p_len;
        let mut pt = make_tree(p_len);

        while tree_len > 0 {
            let start = gen.next_u64() as usize % (tree_len + 1);
            let end = (start + 15).min(tree_len);
            pt.remove(start..end);
            tree_len -= end - start;
            assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        }
    }

    // #[test]
    // fn bug_finder() {
    //     use rand::random;
    //     fn make_tree(
    //         p_len: usize,
    //     ) -> (Tree, ByteBuffer, ByteBuffer) {
    //         let mut tree = Tree::default();
    //         let orig_buf = ByteBuffer::default();
    //         let add_buf = ByteBuffer::default();
    //         insert_piece_to_tree(
    //             &mut tree,
    //             "a".repeat(p_len).to_string().as_str(),
    //             0,
    //             &orig_buf,
    //             &add_buf,
    //         );
    //         (tree, orig_buf, add_buf)
    //     }

    //     let rounds_per_seed = 6;
    //     let mut round = 0;
    //     let mut seed = 0;
    //     let mut gen = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    //     let p_len = 1000;
    //     let (mut tree, mut ob, mut ab) = make_tree(p_len);

    //     loop {
    //         let len = tree.len;
    //         let start = gen.next_u64() as usize % (len + 1);
    //         let end = (start + 15).min(len);
    //         tree.remove(start..end, &ob, &ab);

    //         if let Err(e) = is_valid_tree(&tree) {
    //             println!("=========== ERROR ===============");
    //             tree.print_in_order();
    //             println!("ERROR: {}, seed: {}, round: {}", e, seed, round);
    //             assert!(false);
    //         }

    //         if rounds_per_seed < round || tree.len < p_len / 2 {
    //             let t = make_tree(p_len);
    //             tree = t.0;
    //             ob = t.1;
    //             ab = t.2;

    //             round = 0;
    //             seed += 1;
    //         }

    //         round += 1;
    //     }
    // }

    fn simple_tree() -> PieceTree {
        let mut pt = PieceTree::new();

        let pieces = vec![0, 1, 2];

        // Put pieces in order
        for i in pieces.iter() {
            pt.insert(*i, i.to_string().as_bytes());
            pt.add.extend_from_slice(b"waste");
        }

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert_eq!(3, pt.tree.node_count);
        pt
    }

    fn one_piece_tree() -> PieceTree {
        let mut pt = PieceTree::new();
        pt.insert(0, "abcdefghij");
        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert_eq!(1, pt.tree.node_count);
        pt
    }

    fn complex_tree() -> PieceTree {
        let mut pt = PieceTree::new();

        pt.insert(0, "abcde");
        pt.insert(2, "ab");

        // create gap
        pt.add.extend_from_slice(b"123");

        pt.insert(7, "ab");
        pt.insert(7, "ab");
        pt.insert(7, "ab");
        pt.insert(7, "ab");
        pt.insert(9, "ab");
        pt.insert(9, "ab");

        assert_eq!(Ok(()), is_valid_tree(&pt.tree));
        assert!(!pt.tree.root.is_leaf());
        assert_eq!(9, pt.tree.node_count);

        pt
    }

    impl Tree {
        #[allow(dead_code)]
        pub(crate) fn print_in_order(&self) {
            fn print(node: &Rc<Node>, mut space: usize) {
                space += 10;
                if let Node::Internal(node) = node.as_ref() {
                    print(&node.right, space);
                    println!();
                    print!("{}", " ".repeat(space - 10));
                    println!(
                        "{:?}, {:?}, {:?}",
                        node.color, node.left_subtree_len, node.piece
                    );
                    print(&node.left, space);
                }
            }

            print(&self.root, 0);
        }

        #[allow(dead_code)]
        pub(crate) fn log_in_order(&self) {
            fn print(node: &Rc<Node>, mut space: usize) {
                space += 10;
                if let Node::Internal(node) = node.as_ref() {
                    print(&node.right, space);
                    log::debug!(
                        "{}{:?}, {:?}, {:?}",
                        " ".repeat(space - 10),
                        node.color,
                        node.left_subtree_len,
                        node.piece
                    );
                    print(&node.left, space);
                }
            }

            log::debug!(
                " =========== TREE LOG {:?} =============",
                is_valid_tree(self)
            );
            print(&self.root, 0);
            log::debug!(" =========== TREE END =============");
        }
    }

    fn is_black_height_balanced(node: &Rc<Node>) -> bool {
        fn black_height(node: &Rc<Node>) -> Result<usize, ()> {
            match node.as_ref() {
                Node::Leaf => Ok(1),
                Node::BBLeaf => Ok(2),
                Node::Internal(node) => {
                    let left = black_height(&node.left)?;
                    let right = black_height(&node.right)?;
                    if left == right {
                        Ok(left + if node.color == Color::Black { 1 } else { 0 })
                    } else {
                        Err(())
                    }
                }
            }
        }

        black_height(node).is_ok()
    }

    fn left_subtree_lengths_match(node: &Rc<Node>) -> bool {
        fn subtree_len(node: &Rc<Node>) -> Result<usize, ()> {
            match node.as_ref() {
                Node::Leaf => Ok(0),
                Node::BBLeaf => Ok(0),
                Node::Internal(node) => {
                    let left = subtree_len(&node.left)?;
                    let right = subtree_len(&node.right)?;
                    if left == node.left_subtree_len {
                        Ok(left + right + node.piece.len)
                    } else {
                        Err(())
                    }
                }
            }
        }

        subtree_len(node).is_ok()
    }

    fn red_nodes_have_black_children(node: &Rc<Node>) -> bool {
        let self_ok = if node.color() == Color::Red {
            let node = if let Node::Internal(n) = node.as_ref() {
                n
            } else {
                unreachable!();
            };
            let left = node.left.color();
            let right = node.right.color();
            left == Color::Black && right == Color::Black
        } else {
            true
        };

        if let Node::Internal(n) = node.as_ref() {
            self_ok
                && red_nodes_have_black_children(&n.left)
                && red_nodes_have_black_children(&n.right)
        } else {
            self_ok
        }
    }

    pub(crate) fn is_valid_tree(tree: &Tree) -> Result<(), &'static str> {
        let root = &tree.root;
        if root.color() != Color::Black {
            return Err("Root is not black.");
        }

        if !is_black_height_balanced(&root) {
            return Err("Black height unbalanced.");
        }

        if !red_nodes_have_black_children(&root) {
            return Err("Red nodes have red children.");
        }

        if !left_subtree_lengths_match(&root) {
            return Err("Left subtree counts are invalid.");
        }

        Ok(())
    }
}
