mod memotable;
mod rule;

use std::{
    cmp::min,
    collections::{BinaryHeap, HashMap, HashSet},
};

use thiserror::Error;

use crate::{
    grammar::{self, Rule, RuleDefinition},
    parser::rule::preprocess_rules,
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
    rules: Box<[Clause]>,
}

impl PikaParser {
    pub fn new(grammar: &str) -> Result<PikaParser, ParseError> {
        match grammar::parse_rules_from_str(grammar) {
            Ok(rules) => {
                let prules = preprocess_rules(&rules);
                todo!()
                // let parser = PikaParser { rules:  };
                // Ok(parser)
            }
            Err(e) => Err(ParseError::Grammar(e.to_string())),
        }
    }

    pub fn parse(&self, input: &str) {
        let mut memo = MemoTable::new();
        // Max priority queue
        let mut queue = BinaryHeap::new();
        // TODO these should be
        let terminals: Vec<&Clause> = self
            .rules
            .iter()
            .filter(|c| c.is_terminal() && !c.is_nothing())
            .collect();

        let len = input.chars().count();

        // Match from terminals up
        for (i, ch) in input.chars().rev().enumerate() {
            let pos = len - i;
            terminals.iter().for_each(|p| queue.push(*p));

            while let Some(clause) = queue.pop() {
                let key = MemoKey {
                    clause: clause.order,
                    start: pos,
                };

                // if let Some(mat) = self.try_match(key, &memo, input) {}

                // var memoKey = new MemoKey(clause, startPos);
                // var match = clause.match(memoTable, memoKey, input);
                // memoTable.addMatch(memoKey, match, priorityQueue);
            }
        }
    }

    // fn try_match(&self, key: MemoKey, memo: &MemoTable, input: &str) -> Option<Match> {
    //     let mut rule = &self.rules[key.rule];
    //     let clause = &rule.rule.def;

    //     use RuleDefinition::*;
    //     match clause {
    //         OneOrMore(_) => todo!(),
    //         Choice(_) => todo!(),
    //         Sequence(seq) => {
    //             let mut pos = key.start;
    //             for clause in seq {}
    //             todo!()
    //         }
    //         FollowedBy(_) => todo!(),
    //         NotFollowedBy(_) => todo!(),
    //         CharSequence(seq) => {
    //             let max = min(key.start + seq.len(), input.len());
    //             let slice = &input[key.start..max];
    //             if slice == seq {
    //                 Some(Match {
    //                     key,
    //                     len: seq.len(),
    //                 })
    //             } else {
    //                 None
    //             }
    //         }
    //         Ref(r) => {
    //             let mkey = MemoKey {
    //                 rule: *r,
    //                 start: key.start,
    //             };
    //             self.try_match(mkey, memo, input)
    //         }
    //         Nothing => Some(Match { key, len: 0 }),
    //     }
    // }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parser_calc() {
        let peg = include_str!("../pegs/calc.peg");
        let parser = PikaParser::new(peg);
        // assert!(parser.is_ok());
        // parser.unwrap().parse("1 + 2");
    }
}
