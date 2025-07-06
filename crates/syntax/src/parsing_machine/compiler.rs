use crate::{
    grammar::{self, Rule, Rules},
    parsing_machine::set::Set,
};

use super::{op::Operation, Addr};

pub struct Program {
    pub(crate) ops: Vec<Operation>,
}

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, op) in self.ops.iter().enumerate() {
            write!(f, "{i}: {op:?} ")?;

            writeln!(f)?;
        }

        Ok(())
    }
}

pub(crate) struct Compiler<'a> {
    program: Vec<Operation>,
    rule_addrs: Box<[usize]>,
    rules: &'a Rules,
    rule_head: usize,
}

impl<'a> Compiler<'a> {
    pub fn new(rules: &Rules) -> Compiler {
        Compiler {
            program: Vec::new(),
            rule_addrs: vec![0; rules.len()].into(),
            rules,
            rule_head: 0,
        }
    }

    pub(crate) fn compile_unanchored(mut self) -> anyhow::Result<Program> {
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

        // Unanchor
        //     Choice L1
        //     Call (top)
        //     Commit L2
        // L1: any byte
        //     Jump 0
        // L2: End
        let choice = self.push(Operation::Choice(0));
        self.push(Operation::Call(top));
        let commit = self.push(Operation::Commit(0));
        self.push(Operation::Any(1));
        self.push(Operation::Jump(0));
        let end = self.push(Operation::End);

        self.set_address(choice, end);
        self.set_address(commit, end);

        // Compile all the other rules
        for (i, rule) in self.rules.iter().enumerate() {
            let show = rule.show();

            self.rule_addrs[i] = self.program.len();
            self.rule_head = self.program.len();

            // Add capture if we want to show this in AST
            if show {
                self.push(Operation::CaptureBegin(i));
            }

            // Compile the rule
            self.compile_rec(&rule.rule)?;

            if show {
                self.push(Operation::CaptureEnd);
            }

            // Add a return op
            self.push_return();
        }

        self.finish();

        Ok(Program { ops: self.program })
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
        self.push(Operation::Call(top));
        self.push(Operation::End);

        // Compile all the other rules
        for (i, rule) in self.rules.iter().enumerate() {
            let show = rule.show();

            self.rule_addrs[i] = self.program.len();
            self.rule_head = self.program.len();

            // Add capture if we want to show this in AST
            if show {
                self.push(Operation::CaptureBegin(i));
            }

            // Compile the rule
            self.compile_rec(&rule.rule)?;

            if show {
                self.push(Operation::CaptureEnd);
            }

            // Add a return op
            self.push_return();
        }

        self.finish();

        Ok(Program { ops: self.program })
    }

    fn push_return(&mut self) {
        let prev = self.program.len() - 2;
        let last = self.program.len() - 1;

        match (&self.program[prev], &self.program[last]) {
            // Does not work
            // (Operation::Call(addr), Operation::CaptureEnd) => {
            //     // To enable call, return optimization to just use jump,
            //     // Swap capture and call operation
            //     self.program[last] = Operation::Jump(*addr);
            //     self.program[prev] = Operation::CaptureEnd;
            // }
            (_, Operation::Call(addr)) => {
                self.program[last] = Operation::Jump(*addr);
            }
            _ => {}
        }

        self.push(Operation::Return);
    }

    fn finish(&mut self) {
        self.translate_callsites();
    }

    /// Call/Jump sites initially refer to rules
    /// This translates them to their program offsets
    fn translate_callsites(&mut self) {
        // Set all call sites to their function addresses
        for i in 0..self.program.len() {
            let op = &self.program[i];
            match op {
                Operation::Jump(addr) => {
                    let addr = self.rule_addrs[*addr];
                    self.program[i] = Operation::Jump(addr);
                }
                Operation::Call(addr) => {
                    let addr = self.rule_addrs[*addr];
                    self.program[i] = Operation::Call(addr);
                }
                _ => {}
            }
        }
    }

    fn push(&mut self, op: Operation) -> usize {
        self.program.push(op);
        self.program.len() - 1
    }

    fn set_address(&mut self, at: usize, addr: Addr) {
        let old = match &mut self.program[at] {
            Operation::Jump(a) => a,
            Operation::Call(a) => a,
            Operation::Commit(a) => a,
            Operation::Choice(a) => a,
            Operation::PartialCommit(a) => a,
            Operation::BackCommit(a) => a,
            Operation::TestChar(_, a) => a,
            Operation::TestSet(_, a) => a,
            _ => return,
        };
        *old = addr;
    }

    fn compile_choice_rec(&mut self, rule: &Rule, rest: &[Rule]) -> anyhow::Result<()> {
        //     Choice L2
        //     <rule 1>
        //     Commit L2
        // L1: <rule 2>
        // L2: ...
        if rest.is_empty() {
            self.compile_rec(rule)?;
            return Ok(());
        }

        let choice = self.push(Operation::Choice(0));
        self.compile_rec(rule)?;
        let commit = self.push(Operation::Commit(0));
        self.set_address(choice, self.program.len());

        let (next, nrest) = rest
            .split_first()
            .ok_or(anyhow::anyhow!("Choice no items"))?;
        self.compile_choice_rec(next, nrest)?;

        self.set_address(commit, self.program.len());
        Ok(())
    }

    fn push_span(&mut self, rule: &Rule) -> bool {
        // TODO this slows down all parsing?
        // Is this even correct?
        //
        // if let Rule::ByteRange(a, b) = rule {
        //     let mut set = Set::new();
        //     for i in *a..=*b {
        //         set.add(i);
        //     }

        //     self.push(Operation::Span(set));

        //     return true;
        // }

        false
    }

    fn compile_rec(&mut self, rule: &Rule) -> anyhow::Result<()> {
        use grammar::Rule::*;

        match rule {
            Optional(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule)?;
                let next = self.program.len() + 1;
                self.push(Operation::Commit(next));
                self.set_address(choice, next);
            }
            ZeroOrMore(rule) => {
                //     Choice L2
                // L1: <rule>
                //     PartialCommit L1
                // L2: ...
                if self.push_span(rule) {
                    return Ok(());
                }
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule)?;
                self.push(Operation::PartialCommit(choice + 1));
                let next = self.program.len();
                self.set_address(choice, next);
            }
            OneOrMore(rule) => {
                // One
                self.compile_rec(rule)?;

                // Zero or more
                if self.push_span(rule) {
                    return Ok(());
                }
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule)?;
                self.push(Operation::PartialCommit(choice + 1));
                let next = self.program.len();
                self.set_address(choice, next);
            }
            Choice(rules) => {
                let (first, rest) = rules
                    .split_first()
                    .ok_or(anyhow::anyhow!("Choice with no items"))?;
                self.compile_choice_rec(first, rest)?;
            }
            Sequence(rules) => {
                for rule in rules {
                    self.compile_rec(rule)?;
                }
            }
            FollowedBy(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule)?;
                let bcommit = self.push(Operation::BackCommit(0));
                let fail = self.push(Operation::Fail);
                self.set_address(choice, fail);
                let next = self.program.len();
                self.set_address(bcommit, next);
            }
            NotFollowedBy(rule) => {
                let choice = self.push(Operation::Choice(0));
                self.compile_rec(rule)?;
                self.push(Operation::FailTwice);
                let next = self.program.len();
                self.set_address(choice, next);
            }
            ByteSequence(seq) => {
                for (i, byte) in seq.iter().enumerate() {
                    // Head fail optimization
                    let last = self.program.len() - 1;
                    if i == 0 && self.rule_head == last {
                        if let Operation::Choice(addr) = &self.program[last] {
                            self.program[last] = Operation::TestChar(*byte, *addr);
                            continue;
                        }
                    }

                    self.push(Operation::Byte(*byte));
                }
            }
            UTF8Range(a, b) => {
                self.push(Operation::UTF8Range(*a, *b));
            }
            Ref(idx) => {
                self.push(Operation::Call(*idx));
            }
            ByteRange(a, b) => {
                let mut set = Set::new();
                for i in *a..=*b {
                    set.add(i);
                }

                // Head fail optimization
                let last = self.program.len() - 1;
                if self.rule_head == last {
                    if let Operation::Choice(addr) = &self.program[last] {
                        self.program[last] = Operation::TestSet(set, *addr);
                        return Ok(());
                    }
                }
                self.push(Operation::Set(set));
            }
            ByteAny => {
                self.push(Operation::Any(1));
            }
            Embed(operation) => {
                self.push(operation.clone());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(dead_code)]
    fn print_ops(ops: &[Operation]) {
        println!("------ Operations ---------");
        for (i, op) in ops.iter().enumerate() {
            println!("{i}: {op:?}");
        }
    }

    #[test]
    fn compile_json() {
        let peg = include_str!("../../pegs/json.peg");
        let rules = Rules::parse(std::io::Cursor::new(peg)).unwrap();

        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        println!("{program:?}");
    }

    #[test]
    fn compile_toml() {
        let peg = include_str!("../../pegs/toml.peg");
        let rules = Rules::parse(std::io::Cursor::new(peg)).unwrap();

        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        println!("{program:?}");
    }

    #[test]
    fn compile_brackets() {
        let peg = "WHITESPACE = [ \\t\\r\\n];";
        let rules = Rules::parse(std::io::Cursor::new(peg)).unwrap();

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
        let rules = Rules::parse(std::io::Cursor::new(peg)).unwrap();

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
        let rules = Rules::parse(std::io::Cursor::new(peg)).unwrap();
        println!("{}", rules);

        let compiler = Compiler::new(&rules);
        let program = compiler.compile();
    }
}
