use std::io;

use bitvec::prelude::BitArray;
use rustc_hash::FxHashMap;

use crate::grammar::{self, Rule, RuleDefinition};

type Addr = usize;
type Set = BitArray;

enum Operation {
    Jump(Addr),
    Byte(u8),
    Call(Addr),
    Commit(Addr),
    Choice(Vec<Addr>),
    Any(usize),
    Set(Set),
    Return,
    Fail,
    End,
    EndFail,
}

struct Parser {}

impl Parser {
    pub fn new<R: io::Read>(read: R) -> Parser {
        let rules = grammar::parse_rules(read).unwrap();
        let mut compiler = Compiler::new(&rules);
        compiler.compile();
        todo!()
    }
}

struct Compiler<'a> {
    program: Vec<Operation>,
    map: FxHashMap<usize, usize>,
    rules: &'a [Rule],
}

impl<'a> Compiler<'a> {
    pub fn new(rules: &[Rule]) -> Compiler {
        Compiler {
            program: Vec::new(),
            map: FxHashMap::default(),
            rules,
        }
    }

    pub(crate) fn compile(mut self) -> Vec<Operation> {
        for rule in self.rules {
            if rule.top {
                self.compile_rec(&rule.def);
            }
        }

        self.program
    }

    fn push(&mut self, op: Operation) -> usize {
        self.program.push(op);
        self.program.len() - 1
    }

    fn compile_rec(&mut self, rule: &RuleDefinition) {
        use grammar::RuleDefinition::*;

        match rule {
            Optional(rule) => todo!(),
            ZeroOrMore(rule) => todo!(),
            OneOrMore(rule) => todo!(),
            Choice(rules) => {
                //     Choice L1, L2, L3, ..., Li
                //     <rule 1>
                //     Commit Li
                // L1: <rule 2>
                //     Commit Li
                // L2: <rule 3>
                //     Commit Li
                //   . . .
                // Li: ...
                let mut choices = vec![];
                let choice_op = self.push(Operation::Choice(vec![]));
                let mut commits = vec![];

                for rule in rules {
                    if !commits.is_empty() {
                        choices.push(self.program.len());
                    }
                    self.compile_rec(rule);
                    let commit = self.push(Operation::Commit(0));
                    commits.push(commit);
                }

                self.program[choice_op] = Operation::Choice(choices);
                let next = self.program.len();
                for commit in commits {
                    self.program[commit] = Operation::Commit(next);
                }
            }
            Sequence(rules) => todo!(),
            FollowedBy(rule) => todo!(),
            NotFollowedBy(rule) => todo!(),
            CharSequence(seq) => {
                for byte in seq.as_bytes() {
                    self.push(Operation::Byte(*byte));
                }
            }
            CharRange(a, b) => todo!(),
            Ref(idx) => match self.map.get(&idx) {
                Some(i) => {
                    self.push(Operation::Call(*i));
                }
                None => {
                    let next = self.program.len();
                    self.map.insert(*idx, next);
                    let rule = &self.rules[*idx];
                    self.compile_rec(&rule.def);
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compiler() {
        let peg = include_str!("../pegs/json.peg");
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();

        let mut compiler = Compiler::new(&rules);
        compiler.compile();

        // let parser = PikaParser::from_str(peg).unwrap();
        // let input = " {\"account\":\"bon\",\n\"age\":3.2, \r\n\"children\" : [  1, 2,3], \"allow-children\": true } ";
        // let ast = parser.parse(input).unwrap();
        // ast.print(input);
    }
}
