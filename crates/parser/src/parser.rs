mod memotable;

use std::collections::{BinaryHeap, HashMap, HashSet};

use thiserror::Error;

use crate::{
    grammar::{self, Clause, Rule},
    input::Input,
};

use self::memotable::MemoTable;

struct PikaRule {
    topo_order: usize,
    /// Links to parent rules that reference this rule
    parents: Vec<usize>,
    rule: Rule,
}

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
                println!("Topo: {topo:?}");
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
        // Max priority queue
        let mut queue = BinaryHeap::<usize>::new();
        // TODO these should be
        let terminals: Vec<usize> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(i, r)| {
                let clause = &r.clause;
                clause.is_terminal() && !clause.is_nothing()
            })
            .map(|(i, _)| i)
            .collect();

        // Match from terminals up
        for ch in input.chars().rev() {
            queue.extend(&terminals);

            while let Some(i) = queue.pop() {

                // var memoKey = new MemoKey(clause, startPos);
                // var match = clause.match(memoTable, memoKey, input);
                // memoTable.addMatch(memoKey, match, priorityQueue);
            }
        }
    }
}

fn topological_order(first: usize, rules: &[Rule]) -> Box<[usize]> {
    let mut visited: Box<[bool]> = vec![false; rules.len()].into();
    let mut result = vec![];
    topo_rec(first, rules, &mut visited, &mut result);
    let res: Box<[usize]> = result.into();
    print_in_order(rules, &res);
    res
}

fn topo_rec(idx: usize, rules: &[Rule], visited: &mut Box<[bool]>, result: &mut Vec<usize>) {
    if visited[idx] {
        return;
    }

    visited[idx] = true;
    let rule = &rules[idx];
    let refs = find_refs(&rule.clause);
    for r in refs {
        topo_rec(r, rules, visited, result);
    }
    result.push(idx);
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

fn print_in_order(rules: &[Rule], order: &[usize]) {
    for or in order {
        let rule = &rules[*or];
        println!("{}: {}", *or, rule);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parser_calc() {
        let peg = include_str!("../pegs/calc.peg");
        let parser = PikaParser::new(peg);
    }
}
