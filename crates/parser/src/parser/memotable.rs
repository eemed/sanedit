use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
};

use super::{ast::ASTNode, clause::Clause, ranges::Ranges};

#[derive(Debug)]
pub(crate) struct MemoTable<'a> {
    table: HashMap<MemoKey, Match>,
    pub(crate) clauses: &'a [Clause],
    pub(crate) names: &'a HashMap<usize, Vec<String>>,
}

impl<'a> MemoTable<'a> {
    pub fn new(clauses: &'a [Clause], names: &'a HashMap<usize, Vec<String>>) -> MemoTable<'a> {
        MemoTable {
            table: HashMap::new(),
            names,
            clauses,
        }
    }

    pub fn insert(&mut self, key: MemoKey, mat: Match) -> bool {
        if let Some(o) = self.table.get(&key) {
            if mat.len < o.len {
                return false;
            }
        }

        self.table.insert(key, mat);
        true
    }

    pub fn get(&self, key: &MemoKey) -> Option<Match> {
        match self.table.get(key) {
            Some(m) => Some(m.clone()),
            None => {
                let clause = &self.clauses[key.clause];
                if clause.can_match_zero {
                    Some(Match::empty(key.clone()))
                } else {
                    None
                }
            }
        }
    }

    fn syntax_errors(&self, len: usize) -> Ranges {
        let mut matched = Ranges::new();
        for clause in self.clauses {
            if clause.show {
                for mat in self.non_overlapping_matches(clause.idx) {
                    matched.push(mat.key.start..mat.key.start + mat.len);
                }
            }
        }
        matched.invert(0..len);
        matched
    }

    pub fn to_ast(&self) -> ASTNode {
        let mut len = 0;
        let mut ckey = None;

        for (key, mat) in &self.table {
            let show = self.clauses[key.clause].show;

            if show && mat.len > len {
                len = mat.len;
                ckey = Some(key);
            }
        }

        let k = ckey.expect("No longest match found");
        ASTNode::from_match(k, self)
    }

    fn non_overlapping_matches(&self, clause: usize) -> Vec<&Match> {
        let matches = self.all_matches(clause);
        let mut result = Vec::new();

        for mut i in 0..matches.len() {
            let mat = matches[i];
            let start = mat.key.start;
            let end = start + mat.len;
            result.push(mat);

            while i < matches.len() - 1 && matches[i + 1].key.start < end {
                i += 1;
            }
        }

        result
    }

    fn all_matches(&self, clause: usize) -> Vec<&Match> {
        let mut result = Vec::new();

        for (key, mat) in &self.table {
            if key.clause == clause {
                result.push(mat);
            }
        }

        result
    }

    fn to_map(&self, clause: usize) -> BTreeMap<usize, &Match> {
        let mut map = BTreeMap::new();

        for (key, mat) in &self.table {
            if key.clause == clause {
                map.insert(key.start, mat);
            }
        }

        map
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct MemoKey {
    pub clause: usize,
    /// Input start position
    pub start: usize,
}

impl MemoKey {}

#[derive(Debug, Clone)]
pub(crate) struct Match {
    pub key: MemoKey,

    /// Length of the match
    pub len: usize,

    pub sub: Vec<MemoKey>,
}

impl Match {
    pub fn empty(key: MemoKey) -> Match {
        Match {
            key,
            len: 0,
            sub: vec![],
        }
    }

    pub fn terminal(key: MemoKey, len: usize) -> Match {
        Match {
            key,
            len,
            sub: vec![],
        }
    }
}
