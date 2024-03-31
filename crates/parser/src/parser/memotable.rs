use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::PikaParser;

use super::{ast::AST, clause::ClauseKind};

#[derive(Debug)]
pub(crate) struct MemoTable<'a, 'b> {
    table: FxHashMap<MemoKey, Match>,
    pub(crate) parser: &'a PikaParser,
    pub(crate) input: &'b str,
}

impl<'a, 'b> MemoTable<'a, 'b> {
    pub fn new(parser: &'a PikaParser, input: &'b str) -> MemoTable<'a, 'b> {
        MemoTable {
            table: FxHashMap::default(),
            parser,
            input,
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
                let clause = &self.parser.preproc.clauses[key.clause];
                if clause.kind == ClauseKind::NotFollowedBy {
                    self.parser.try_match(*key, self, self.input)
                } else if clause.can_match_zero {
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
        let clauses = &self.parser.preproc.clauses;

        // TODO optimize
        for (key, mat) in &self.table {
            if key.start < at {
                continue;
            }
            let clause = &clauses[key.clause];
            if !clause.show {
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
                    let cur = clauses[key.clause].order;
                    let prev = result
                        .as_ref()
                        .map(|r| clauses[r.key.clause].order)
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

    pub sub: SmallVec<[MemoKey; 2]>,
}

impl Match {
    pub fn empty(key: MemoKey) -> Match {
        Match {
            key,
            len: 0,
            sub: SmallVec::new(),
        }
    }

    pub fn terminal(key: MemoKey, len: usize) -> Match {
        Match {
            key,
            len,
            sub: SmallVec::new(),
        }
    }
}
