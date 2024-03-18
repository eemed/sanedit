use std::ops::Range;

use super::memotable::{Match, MemoKey, MemoTable};

#[derive(Debug, Clone)]
pub struct AST {
    label: String,
    start: usize,
    len: usize,
    sub: Vec<AST>,
}

impl AST {
    pub(crate) fn new(memo: &MemoTable, len: usize) -> AST {
        const ERROR_LABEL: &str = "error";
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
            roots.push(node);
            pos += mat.len;
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
        fn rec(node: &mut AST, key: &MemoKey, memo: &MemoTable) {
            let mat = memo.get(key).unwrap();
            for sub in &mat.sub {
                let smat = memo.get(sub).unwrap();
                let show = memo.parser.preproc.clauses[smat.key.clause].show;
                if show {
                    node.sub.push(AST::from_match(&smat, memo))
                } else {
                    rec(node, &smat.key, memo)
                }
            }
        }

        let name = memo
            .parser
            .preproc
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

        rec(&mut node, &mat.key, memo);
        node
    }

    pub fn print(&self, input: &str) {
        fn print_rec(node: &AST, input: &str, level: usize) {
            println!(
                "{}{}: {:?}",
                " ".repeat(level),
                node.label,
                &input[node.start..node.start + node.len]
            );
            for s in &node.sub {
                print_rec(s, input, level + 2);
            }
        }

        print_rec(self, input, 0);
    }

    pub fn print_string(&self, input: &str) -> String {
        fn print_string_rec(node: &AST, input: &str, level: usize) -> String {
            let mut res = format!(
                "{}{}: {:?}",
                " ".repeat(level),
                node.label,
                &input[node.start..node.start + node.len]
            );
            for s in &node.sub {
                let next = print_string_rec(s, input, level + 2);
                res.push_str("\n");
                res.push_str(&next);
            }

            res
        }

        print_string_rec(self, input, 0)
    }

    pub fn flatten(&self) -> Vec<AST> {
        let mut stack = vec![];
        let mut result = vec![];
        stack.push(self);

        while let Some(n) = stack.pop() {
            for sub in &n.sub {
                stack.push(sub);
            }
            result.push(n.clone());
        }

        result
    }

    pub fn name(&self) -> &str {
        &self.label
    }

    pub fn range(&self) -> Range<usize> {
        self.start..self.start + self.len
    }
}
