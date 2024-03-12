use std::collections::{BTreeMap, HashMap};

use super::clause::Clause;

#[derive(Debug)]
pub(crate) struct MemoTable<'a> {
    table: HashMap<MemoKey, Match>,
    clauses: &'a [Clause],
}

impl<'a> MemoTable<'a> {
    pub fn new(clauses: &'a [Clause]) -> MemoTable<'a> {
        MemoTable {
            table: HashMap::new(),
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
                    Some(Match {
                        key: key.clone(),
                        len: 0,
                    })
                } else {
                    None
                }
            }
        }
    }

    pub fn to_ast(&self, clause: usize, input: &str) {
        for mat in self.non_overlapping_matches(clause) {
            let start = mat.key.start;
            let end = start + mat.len;
            // println!("Match: {mat:?}");
            println!("Matched text: {}", &input[start..end]);
        }
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
}

impl Match {}
