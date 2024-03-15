use super::memotable::{MemoKey, MemoTable};

#[derive(Debug)]
pub(crate) struct ASTNode {
    label: String,
    start: usize,
    len: usize,
    sub: Vec<ASTNode>,
}

impl ASTNode {
    pub fn from_match(key: &MemoKey, memo: &MemoTable) -> ASTNode {
        let mat = memo.get(key).unwrap();
        let names = memo.names.get(&mat.key.clause).unwrap();
        let name = &names[0];
        let mut node = ASTNode {
            label: name.clone(),
            start: mat.key.start,
            len: mat.len,
            sub: vec![],
        };

        Self::rec(&mut node, key, memo);
        node
    }

    fn rec(node: &mut ASTNode, key: &MemoKey, memo: &MemoTable) {
        let mat = memo.get(key).unwrap();
        for sub in &mat.sub {
            let smat = memo.get(sub).unwrap();
            let show = memo.clauses[smat.key.clause].show;
            if show {
                node.sub.push(ASTNode::from_match(&smat.key, memo))
            } else {
                Self::rec(node, &smat.key, memo)
            }
        }
    }

    pub fn print(&self, input: &str) {
        Self::print_rec(self, input, 0);
    }

    fn print_rec(node: &ASTNode, input: &str, level: usize) {
        println!(
            "{}{}: {:?}",
            " ".repeat(level),
            node.label,
            &input[node.start..node.start + node.len]
        );
        for s in &node.sub {
            Self::print_rec(s, input, level + 2);
        }
    }
}
