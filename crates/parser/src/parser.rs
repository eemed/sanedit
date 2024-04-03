mod ast;
mod clause;
mod memotable;
mod preprocess;
mod set;

use std::{cmp::min, collections::BinaryHeap, io};

use smallvec::SmallVec;
use thiserror::Error;

use crate::{grammar, parser::clause::ClauseKind};

pub use self::ast::AST;
use self::{
    clause::Clause,
    memotable::{Match, MemoKey, MemoTable},
    preprocess::{preprocess_rules, Clauses},
};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse grammar: {0}")]
    Grammar(String),

    #[error("Failed to preprocess rules: {0}")]
    Preprocess(String),
}

// https://arxiv.org/pdf/2005.06444.pdf
#[derive(Debug)]
pub struct PikaParser {
    preproc: Clauses,
}

impl PikaParser {
    pub fn new<R: io::Read>(read: R) -> Result<PikaParser, ParseError> {
        let rules = grammar::parse_rules(read).map_err(|o| ParseError::Grammar(o.to_string()))?;
        let clauses =
            preprocess_rules(&rules).map_err(|o| ParseError::Preprocess(o.to_string()))?;
        let parser = PikaParser { preproc: clauses };
        Ok(parser)
    }

    pub fn from_str(grammar: &str) -> Result<PikaParser, ParseError> {
        let rules = grammar::parse_rules_from_str(grammar)
            .map_err(|o| ParseError::Grammar(o.to_string()))?;
        let clauses =
            preprocess_rules(&rules).map_err(|o| ParseError::Preprocess(o.to_string()))?;
        let parser = PikaParser { preproc: clauses };
        Ok(parser)
    }

    pub fn parse(&self, input: &str) -> AST {
        let mut memo = MemoTable::new(&self, input);
        // Max priority queue
        let mut queue = BinaryHeap::new();
        let terminals: Vec<&Clause> = self
            .preproc
            .clauses
            .iter()
            .filter(|c| c.is_terminal() && !c.is_nothing())
            .collect();

        let len = input.len();

        // Match from terminals up
        for pos in (0..len).rev() {
            terminals.iter().for_each(|p| queue.push(*p));

            while let Some(clause) = queue.pop() {
                let i = clause.idx;
                let key = MemoKey {
                    clause: i,
                    start: pos,
                };

                if let Some(mat) = self.try_match(key, &memo, input) {
                    let updated = memo.insert(key, mat);
                    for parent in clause.parents.iter().map(|i| &self.preproc.clauses[*i]) {
                        if updated || parent.can_match_zero {
                            queue.push(parent);
                        }
                    }
                }
            }
        }

        let len = input.len();
        memo.to_ast(len)
    }

    pub(crate) fn try_match(&self, key: MemoKey, memo: &MemoTable, input: &str) -> Option<Match> {
        use ClauseKind::*;

        let clause = &self.preproc.clauses[key.clause];
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
                        sub: SmallVec::from_slice(&[skey, tail_key]),
                    }),
                    None => Some(Match {
                        key,
                        len: mat.len,
                        sub: SmallVec::from_slice(&[skey]),
                    }),
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
                        return Some(Match {
                            key,
                            len: mat.len,
                            sub: SmallVec::from_slice(&[skey]),
                        });
                    }
                }

                None
            }
            Sequence => {
                let mut subs = SmallVec::with_capacity(clause.sub.len());
                let mut pos = key.start;
                for sub in &clause.sub {
                    let skey = MemoKey {
                        clause: *sub,
                        start: pos,
                    };
                    let mat = memo.get(&skey)?;
                    subs.push(skey);
                    pos += mat.len;
                }

                Some(Match {
                    key,
                    len: pos - key.start,
                    sub: subs,
                })
            }
            CharSequence(seq) => {
                let max = min(key.start + seq.len(), input.len());
                let slice = &input[key.start..max];
                if slice == seq {
                    Some(Match::terminal(key, seq.len()))
                } else {
                    None
                }
            }
            Nothing => Some(Match::empty(key)),
            FollowedBy => {
                let sub = clause.sub[0];
                let skey = MemoKey {
                    clause: sub,
                    start: key.start,
                };
                let _mat = memo.get(&skey)?;
                Some(Match::empty(key))
            }
            NotFollowedBy => {
                let sub = clause.sub[0];
                let skey = MemoKey {
                    clause: sub,
                    start: key.start,
                };

                let mat = memo
                    .get(&skey)
                    .or_else(|| self.try_match(skey, memo, input));

                if mat.is_none() {
                    Some(Match::empty(key))
                } else {
                    None
                }
            }
            CharRange(a, b) => {
                let ch = &input[key.start..].chars().next().unwrap();
                if a <= ch && ch <= b {
                    Some(Match::terminal(key, ch.len_utf8()))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parser_calc() {
        let peg = include_str!("../pegs/calc.peg");
        let parser = PikaParser::from_str(peg).unwrap();
        parser.parse("( 1 + 2 ) * 4");
    }

    #[test]
    fn parser_simple() {
        let peg = include_str!("../pegs/simple.peg");
        let parser = PikaParser::from_str(peg).unwrap();
        parser.parse("1+2^2");
    }

    #[test]
    fn parser_json() {
        let peg = include_str!("../pegs/json.peg");
        let parser = PikaParser::from_str(peg).unwrap();
        let input = " {\"account\":\"bon\",\n\"age\":3.2, \r\n\"children\" : [  1, 2,3], \"allow-children\": true } ";
        let ast = parser.parse(input);
        ast.print(input);
    }

    #[test]
    fn parser_invalid_json() {
        let peg = include_str!("../pegs/json.peg");
        let parser = PikaParser::from_str(peg).unwrap();
        let input = " {\"account\":\"bon\",\n\"age\":3.2 \r\n\"children\" : [  1, 2,3], \"allow-children\": true } ";
        let ast = parser.parse(input);
        ast.print(input);
    }
}
