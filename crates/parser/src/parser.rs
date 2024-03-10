mod memotable;
mod rule;

use std::{
    cmp::min,
    collections::{BinaryHeap, HashMap, HashSet},
};

use thiserror::Error;

use crate::{
    grammar::{self, Rule, RuleDefinition},
    parser::rule::{preprocess_rules, ClauseKind},
};

use self::{
    memotable::{Match, MemoKey, MemoTable},
    rule::{Clause, PikaRule},
};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse grammar: {0}")]
    Grammar(String),
}

// https://arxiv.org/pdf/2005.06444.pdf
#[derive(Debug)]
pub struct PikaParser {
    clauses: Box<[Clause]>,
}

impl PikaParser {
    pub fn new(grammar: &str) -> Result<PikaParser, ParseError> {
        match grammar::parse_rules_from_str(grammar) {
            Ok(rules) => {
                let clauses = preprocess_rules(&rules);
                let parser = PikaParser { clauses };
                Ok(parser)
            }
            Err(e) => Err(ParseError::Grammar(e.to_string())),
        }
    }

    pub fn parse(&self, input: &str) {
        let mut memo = MemoTable::new();
        // Max priority queue
        let mut queue = BinaryHeap::new();
        let terminals: Vec<(usize, &Clause)> = self
            .clauses
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_terminal() && !c.is_nothing())
            .collect();

        let len = input.len();

        // Match from terminals up
        for pos in (0..len).rev() {
            terminals.iter().for_each(|p| queue.push(*p));

            while let Some((i, clause)) = queue.pop() {
                let key = MemoKey {
                    clause: i,
                    start: pos,
                };

                if let Some(mat) = self.try_match(key, &memo, input) {
                    memo.insert(key, mat);
                    let parents: Vec<(usize, &Clause)> = clause
                        .parents
                        .iter()
                        .map(|i| (*i, &self.clauses[*i]))
                        .collect();
                    queue.extend(parents);
                }
            }
        }

        println!("Memo: {memo:?}");
    }

    fn try_match(&self, key: MemoKey, memo: &MemoTable, input: &str) -> Option<Match> {
        use ClauseKind::*;

        let clause = &self.clauses[key.clause];
        match &clause.kind {
            OneOrMore => {
                let sub = clause.sub[0];
                let skey = MemoKey {
                    clause: sub,
                    start: key.start,
                };
                let mat = memo.get(&skey)?;
                let tail_key = MemoKey {
                    clause: key.clause,
                    start: key.start + mat.len,
                };

                match memo.get(&tail_key) {
                    Some(t) => Some(Match {
                        key,
                        len: mat.len + t.len,
                    }),
                    None => Some(Match { key, len: mat.len }),
                }
            }
            Choice => {
                let pos = key.start;
                for sub in &clause.sub {
                    let skey = MemoKey {
                        clause: *sub,
                        start: pos,
                    };
                    if let Some(mat) = memo.get(&skey) {
                        return Some(Match { key, len: mat.len });
                    }
                }

                None
            }
            Sequence => {
                let mut pos = key.start;
                for sub in &clause.sub {
                    let skey = MemoKey {
                        clause: *sub,
                        start: pos,
                    };
                    let mat = memo.get(&skey)?;
                    pos += mat.len;
                }

                Some(Match {
                    key,
                    len: pos - key.start,
                })
            }
            CharSequence(seq) => {
                let max = min(key.start + seq.len(), input.len());
                let slice = &input[key.start..max];
                if slice == seq {
                    Some(Match {
                        key,
                        len: seq.len(),
                    })
                } else {
                    None
                }
            }
            Nothing => Some(Match { key, len: 0 }),
            FollowedBy => todo!(),
            NotFollowedBy => todo!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parser_calc() {
        let peg = include_str!("../pegs/calc.peg");
        let parser = PikaParser::new(peg);
        assert!(parser.is_ok());
        parser.unwrap().parse("1 + 2");
    }
}
