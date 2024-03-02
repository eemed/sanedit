mod memotable;

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{
    grammar::{self, Clause, Rule},
    input::Input,
};

use self::memotable::MemoTable;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse grammar: {0}")]
    Grammar(String),
}

// https://arxiv.org/pdf/2005.06444.pdf
#[derive(Debug)]
pub struct PikaParser {
    rules: Box<[Rule]>,
    topological_order: Box<[usize]>,
}

impl PikaParser {
    pub fn new(grammar: &str) -> Result<PikaParser, ParseError> {
        match grammar::parse_rules_from_str(grammar) {
            Ok(rules) => {
                let topo = topological_order(0, &rules);
                let parser = PikaParser {
                    rules,
                    topological_order: topo,
                };
                Ok(parser)
            }
            Err(e) => Err(ParseError::Grammar(e.to_string())),
        }
    }

    pub fn parse(&mut self, input: &str) {
        let mut memo = MemoTable::new();

        for ch in input.chars().rev() {}
    }
}

fn topological_order(first: usize, rules: &[Rule]) -> Box<[usize]> {
    // TODO use recursion
    //
    // let mut postorder = vec![];
    // let mut visited = vec![false; rules.len()];
    // let mut stack = vec![];
    // stack.push(first);

    // while !stack.is_empty() {
    //     // Peek
    //     let cur = stack.last().copied().unwrap();
    //     let mut tail = true;

    //     let rule = &rules[cur];
    //     let refs = find_refs(&rule.clause);
    //     for r in refs {
    //         if !visited[r] {
    //             tail = false;
    //             visited[r] = true;
    //             stack.push(r);
    //             break;
    //         }
    //     }

    //     if tail {
    //         stack.pop();
    //         postorder.push(cur)
    //     }
    // }

    // postorder.into()
}

fn find_refs(clause: &Clause) -> HashSet<usize> {
    use Clause::*;
    match clause {
        OneOrMore(r) | FollowedBy(r) | NotFollowedBy(r) => find_refs(r),
        Choice(v) | Sequence(v) => v.iter().fold(HashSet::new(), |mut acc, c| {
            acc.extend(&find_refs(c));
            acc
        }),
        Ref(i) => {
            let mut set = HashSet::new();
            set.insert(*i);
            set
        }
        _ => HashSet::new(),
    }
}
