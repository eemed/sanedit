use super::memotable::{Match, MemoKey, MemoTable};

#[derive(Debug)]
pub struct AST {
    label: String,
    start: usize,
    len: usize,
    sub: Vec<AST>,
}

impl AST {
    pub(crate) fn new(memo: &MemoTable, len: usize) -> AST {
        const ERROR_LABEL: &str = "ERROR";
        let mut pos = 0;
        let mut roots = vec![];

        while let Some(mat) = memo.best_match_at(pos) {
            // If we left a gap, create error node
            let start = mat.key.start;
            if start != pos {
                roots.push(AST {
                    label: ERROR_LABEL.into(),
                    start: pos,
                    len: start - pos,
                    sub: vec![],
                });
                pos += start - pos;
            }

            let node = Self::from_match(&mat, memo);
            pos += node.len;
            roots.push(node);
        }

        if pos != len {
            roots.push(AST {
                label: ERROR_LABEL.into(),
                start: pos,
                len: len - pos,
                sub: vec![],
            });
        }

        if roots.len() == 1 {
            roots.pop().unwrap()
        } else {
            AST {
                label: "<root>".into(),
                start: 0,
                len,
                sub: roots,
            }
        }
    }

    pub(crate) fn from_match(mat: &Match, memo: &MemoTable) -> AST {
        let name = memo
            .names
            .get(&mat.key.clause)
            .map(|n| n.get(0))
            .flatten()
            .map(String::as_str)
            .unwrap_or("<unkown>");

        let mut node = AST {
            label: name.into(),
            start: mat.key.start,
            len: mat.len,
            sub: vec![],
        };

        Self::rec(&mut node, &mat.key, memo);
        node
    }

    fn rec(node: &mut AST, key: &MemoKey, memo: &MemoTable) {
        let mat = memo.get(key).unwrap();
        for sub in &mat.sub {
            let smat = memo.get(sub).unwrap();
            let show = memo.clauses[smat.key.clause].show;
            if show {
                node.sub.push(AST::from_match(&smat, memo))
            } else {
                Self::rec(node, &smat.key, memo)
            }
        }
    }

    pub fn print(&self, input: &str) {
        Self::print_rec(self, input, 0);
    }

    fn print_rec(node: &AST, input: &str, level: usize) {
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

    pub fn print_string(&self, input: &str) -> String {
        Self::print_string_rec(self, input, 0)
    }

    fn print_string_rec(node: &AST, input: &str, level: usize) -> String {
        let mut res = format!(
            "{}{}: {:?}",
            " ".repeat(level),
            node.label,
            &input[node.start..node.start + node.len]
        );
        for s in &node.sub {
            let next = Self::print_string_rec(s, input, level + 2);
            res.push_str("\n");
            res.push_str(&next);
        }

        res
    }
}
