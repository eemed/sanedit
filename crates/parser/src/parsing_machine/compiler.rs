use std::collections::BTreeMap;

use crate::{
    grammar::{self, Rule, Rules},
    parsing_machine::set::Set,
};

use super::op::Operation;

pub(crate) struct Program {
    pub(crate) ops: Vec<Operation>,
    pub(crate) names: BTreeMap<usize, String>,
}

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, op) in self.ops.iter().enumerate() {
            write!(f, "{i}: {op:?} ")?;

            if let Some(name) = self.names.get(&i) {
                write!(f, " <- {name}")?;
            }
            writeln!(f, "")?;
        }

        Ok(())
    }
}

pub(crate) struct Compiler<'a> {
    program: Vec<Operation>,
    call_sites: Vec<(usize, usize)>,
    rules: &'a Rules,
}

impl<'a> Compiler<'a> {
    pub fn new(rules: &Rules) -> Compiler {
        Compiler {
            program: Vec::new(),
            call_sites: Vec::new(),
            rules,
        }
    }

    pub(crate) fn compile(mut self) -> anyhow::Result<Program> {
        let top = {
            let mut result = 0;
            for (i, rule) in self.rules.iter().enumerate() {
                if rule.top {
                    result = i;
                    break;
                }
            }

            result
        };

        // Push top rule call
        let site = self.push(Operation::Call(0));
        self.call_sites.push((top, site));
        self.push(Operation::End);

        let mut compile_addrs = vec![0; self.rules.len()];

        // Compile all the other rules
        for (i, rule) in self.rules.iter().enumerate() {
            let show = rule.show();

            compile_addrs[i] = self.program.len();

            // Add capture if we want to show this in AST
            if show {
                self.push(Operation::CaptureBegin(i));
            }

            // Compile the rule
            self.compile_rec(&rule.rule);

            if show {
                self.push(Operation::CaptureEnd);
            }

            // Add a return op
            self.push(Operation::Return);
        }

        // Program addresses to names mapping
        let mut names = BTreeMap::default();

        // Set all call sites to their function addresses
        for (rule, site) in &self.call_sites {
            let addr = compile_addrs[*rule];
            self.program[*site] = Operation::Call(addr);
            names.insert(addr, self.rules[*rule].name.clone());
        }

        Ok(Program {
            ops: self.program,
            names,
        })
    }

    fn push(&mut self, op: Operation) -> usize {
        self.program.push(op);
        self.program.len() - 1
    }

    fn compile_choice_rec(&mut self, rule: &Rule, rest: &[Rule]) {
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

    fn compile_rec(&mut self, rule: &Rule) {
        use grammar::Rule::*;

        match rule {
            Optional(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                let next = self.program.len() + 1;
                self.push(Operation::Commit(next));
                self.program[choice] = Operation::Choice(next);
            }
            ZeroOrMore(rule) => {
                //     Choice L2
                // L1: <rule>
                //     PartialCommit L1
                // L2: ...
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                self.push(Operation::PartialCommit(choice + 1));
                let next = self.program.len();
                self.program[choice] = Operation::Choice(next);
            }
            OneOrMore(rule) => {
                // One
                self.compile_rec(rule);

                // Zero or more
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule);
                self.push(Operation::PartialCommit(choice + 1));
                let next = self.program.len();
                self.program[choice] = Operation::Choice(next);
            }
            Choice(rules) => {
                let (first, rest) = rules.split_first().unwrap();
                self.compile_choice_rec(first, rest);
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
            ByteSequence(seq) => {
                for byte in seq {
                    self.push(Operation::Byte(*byte));
                }
            }
            UTF8Range(a, b) => {
                self.push(Operation::UTF8Range(*a, *b));
            }
            Ref(idx) => {
                let site = self.push(Operation::Call(0));
                self.call_sites.push((*idx, site));
            }
            ByteRange(a, b) => {
                let mut set = Set::new();
                for i in *a..=*b {
                    set.add(i);
                }
                self.push(Operation::Set(set));
            }
            ByteAny => {
                self.push(Operation::Set(Set::any()));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn print_ops(ops: &[Operation]) {
        println!("------ Operations ---------");
        for (i, op) in ops.iter().enumerate() {
            println!("{i}: {op:?}");
        }
    }

    #[test]
    fn compile_json() {
        let peg = include_str!("../../pegs/json.peg");
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();

        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        println!("{program:?}");
    }

    #[test]
    fn compile_toml() {
        let peg = include_str!("../../pegs/toml.peg");
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();

        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        println!("{program:?}");
    }

    #[test]
    fn compile_brackets() {
        let peg = "WHITESPACE = [ \\t\\r\\n];";
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();

        let compiler = Compiler::new(&rules);
        let program = compiler.compile();

        println!("{program:?}");
    }

    #[test]
    fn compile_small() {
        let peg = "
            document = _ value _;
            WHITESPACE = [ \\t\\r\\n];
            _ = WHITESPACE*;
            @show
            value = \"abba\";
            ";
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();

        let compiler = Compiler::new(&rules);
        let program = compiler.compile();
        println!("{program:?}");
    }

    #[test]
    fn compile_recovery_small() {
        let peg = "
            document = _ value _;
            WHITESPACE = [ \\t\\r\\n];
            _ = WHITESPACE*;
            value = \"abba\";
            single = [\\uff];
            range = [\\u00..\\u20];
            combi = [\\u0..\\u20\\u25];
            ";
        let rules = grammar::parse_rules(std::io::Cursor::new(peg)).unwrap();
        println!("{}", rules);

        let compiler = Compiler::new(&rules);
        let program = compiler.compile();
    }
}
