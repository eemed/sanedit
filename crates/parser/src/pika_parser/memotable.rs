use std::borrow::Cow;

use rustc_hash::FxHashMap;

use crate::{byte_reader::ByteReader, PikaParser};

use super::{ast::AST, clause::ClauseKind};

#[derive(Debug)]
pub(crate) struct MemoTable<'a, 'b, B: ByteReader> {
    table: FxHashMap<MemoKey, Match>,
    pub(crate) parser: &'a PikaParser,
    pub(crate) input: &'b B,
}

impl<'a, 'b, B: ByteReader> MemoTable<'a, 'b, B> {
    pub fn new(parser: &'a PikaParser, input: &'b B) -> MemoTable<'a, 'b, B> {
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

    pub fn get(&self, key: &MemoKey) -> Option<Cow<Match>> {
        match self.table.get(key) {
            Some(m) => Some(Cow::Borrowed(m)),
            None => {
                let clause = &self.parser.preproc.clauses[key.clause];
                if clause.kind == ClauseKind::NotFollowedBy {
                    self.parser
                        .try_match(*key, self, self.input)
                        .map(Cow::Owned)
                } else if clause.can_match_zero {
                    Some(Cow::Owned(Match::empty()))
                } else {
                    None
                }
            }
        }
    }

    pub fn to_ast(&self, len: usize) -> AST {
        AST::new(self, len)
    }

    pub fn best_match_at(&self, at: usize) -> Option<(&MemoKey, &Match)> {
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
            if !clause.top {
                continue;
            }
            let proximity = key.start - at;

            if proximity < prox {
                result = Some((key, mat));
                prox = proximity;
                len = mat.len;
            } else if proximity == prox {
                if mat.len > len {
                    result = Some((key, mat));
                    len = mat.len;
                }
                // else if mat.len == len {
                // let cur = clauses[key.clause].order;
                // let prev = result
                //     .as_ref()
                //     .map(|r| clauses[r.key.clause].order)
                //     .unwrap_or(usize::MAX);
                // if cur < prev {
                //     result = Some(mat);
                // }
                // }
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

#[derive(Debug, Clone)]
pub(crate) struct Match {
    /// Length of the match
    pub len: usize,

    pub sub: Vec<MemoKey>,
}

impl Match {
    pub fn empty() -> Match {
        Match {
            len: 0,
            sub: Vec::new(),
        }
    }

    pub fn terminal(len: usize) -> Match {
        Match {
            len,
            sub: Vec::new(),
        }
    }
}
