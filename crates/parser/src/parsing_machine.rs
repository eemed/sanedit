use std::io;
use std::ops::{Index, IndexMut};

use bitvec::prelude::*;
use bitvec::{bitarr, prelude::BitArray, BitArr};
use rustc_hash::FxHashMap;

use crate::grammar::{self, Rule, RuleDefinition};

type Addr = usize;

struct Set {
    inner: BitArray<[u32; 8]>,
}

impl Set {
    pub fn new() -> Set {
        Set {
            inner: bitarr![u32, Lsb0; 0; 256],
        }
    }

    pub fn add(&mut self, n: u8) {
        self.inner.set(n as usize, true);
    }
}

impl Index<u8> for Set {
    type Output = bool;

    fn index(&self, index: u8) -> &Self::Output {
        &self.inner[index as usize]
    }
}

impl Index<usize> for Set {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

enum Operation {
    Jump(Addr),
    Byte(u8),
    Call(Addr),
    Commit(Addr),
    Choice(Addr),
    Any(usize),
    Set(Set),
    Return,
    Fail,
    End,
    EndFail,
    PartialCommit(Addr),
    FailTwice,
    Span(Set),
    TestSetNoChoice(Set),
    BackCommit(Addr),
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

    fn compile_choice_rec(&mut self, rule: &RuleDefinition, rest: &[RuleDefinition]) {
        //     Choice L1
        //     <rule 1>
        //     Commit L2
        // L1: <rule 2>
        // L2: ...
        if rest.is_empty() {
            self.compile_rec(rule);
            return;
        }

        let choice = self.push(Operation::Choice(0));
        self.compile_rec(rule);
        let commit = self.push(Operation::Commit(0));
        self.program[choice] = Operation::Choice(self.program.len());

        let (next, nrest) = rest.split_first().unwrap();
        self.compile_choice_rec(next, nrest);

        self.program[commit] = Operation::Commit(self.program.len());
    }

    fn compile_rec(&mut self, rule: &RuleDefinition) {
        use grammar::RuleDefinition::*;

        match rule {
            Optional(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                let next = self.program.len() + 1;
                self.push(Operation::Commit(next));
                self.program[choice] = Operation::Choice(next);
            }
            ZeroOrMore(rule) => {
                // L1: Choice L2
                //     <rule>
                //     PartialCommit L1
                // L2: ...
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                self.push(Operation::PartialCommit(choice));
                let next = self.program.len();
                self.program[choice] = Operation::Choice(next);
            }
            OneOrMore(rule) => {
                // One
                self.compile_rec(rule);

                // Zero or more
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                self.push(Operation::PartialCommit(choice));
                let next = self.program.len();
                self.program[choice] = Operation::Choice(next);
            }
            Choice(rules) => {
                let (first, rest) = rules.split_first().unwrap();
                self.compile_choice_rec(first, rest)
            }
            Sequence(rules) => {
                for rule in rules {
                    self.compile_rec(rule);
                }
            }
            FollowedBy(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                let bcommit = self.push(Operation::BackCommit(0));
                let fail = self.push(Operation::Fail);
                self.program[choice] = Operation::Choice(fail);
                let next = self.program.len();
                self.program[bcommit] = Operation::BackCommit(next);
            }
            NotFollowedBy(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                self.push(Operation::FailTwice);
                let next = self.program.len();
                self.program[choice] = Operation::Choice(next);
            }
            CharSequence(seq) => {
                for byte in seq.as_bytes() {
                    self.push(Operation::Byte(*byte));
                }
            }
            CharRange(a, b) => {
                let mut autf = [0; 4];
                let mut butf = [0; 4];

                a.encode_utf8(&mut autf);
                b.encode_utf8(&mut butf);

                match (a.len_utf8(), b.len_utf8()) {
                    (1, 1) => {
                        let mut set = Set::new();
                        for i in autf[0]..butf[0] {
                            set.add(i);
                        }

                        self.push(Operation::Set(set));
                    }
                    _ => {
                        // TODO
                    }
                }
            }
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
