use std::collections::HashMap;

use super::{ast::AST, clause::Clause};

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

    pub fn to_ast(&self, len: usize) -> AST {
        AST::new(self, len)
    }

    pub fn best_match_at(&self, at: usize) -> Option<&Match> {
        let mut result = None;
        let mut prox = usize::MAX;
        let mut len = 0;

        // TODO optimize
        for (key, mat) in &self.table {
            if key.start < at {
                continue;
            }
            let show = self.clauses[key.clause].show;
            if !show {
                continue;
            }
            let proximity = key.start - at;

            if proximity < prox {
                result = Some(mat);
                prox = proximity;
                len = mat.len;
            } else if proximity == prox {
                if mat.len > len {
                    result = Some(mat);
                    len = mat.len;
                } else if mat.len == len {
                    let cur = self.clauses[key.clause].order;
                    let prev = result
                        .as_ref()
                        .map(|r| self.clauses[r.key.clause].order)
                        .unwrap_or(usize::MAX);
                    if cur < prev {
                        result = Some(mat);
                    }
                }
            }
        }

        result
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
