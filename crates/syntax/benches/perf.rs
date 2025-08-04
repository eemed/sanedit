use criterion::{criterion_group, criterion_main, Criterion};
use sanedit_syntax::bench::{Jit, ParsingMachine};

const RUST: &str = r#"
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
        // TODO this slows down JIT about 10%?
        // interpreted gains about 5%
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
"#;

const LOREM: &str = "
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Maecenas sit amet tellus
nec turpis feugiat semper. Nam at nulla laoreet, finibus eros sit amet, fringilla
mauris. Fusce vestibulum nec ligula efficitur laoreet. Nunc orci leo, varius eget
ligula vulputate, consequat eleifend nisi. Cras justo purus, imperdiet a augue
malesuada, convallis cursus libero. Fusce pretium arcu in elementum laoreet. Duis
mauris nulla, suscipit at est nec, malesuada pellentesque eros. Quisque semper porta
malesuada. Nunc hendrerit est ac faucibus mollis. Nam fermentum id libero sed
egestas. Duis a accumsan sapien. Nam neque diam, congue non erat et, porta sagittis
turpis. Vivamus vitae mauris sit amet massa mollis molestie. Morbi scelerisque,
augue id congue imperdiet, felis lacus euismod dui, vitae facilisis massa dui quis
sapien. Vivamus hendrerit a urna a lobortis.

Donec ut suscipit risus. Vivamus dictum auctor vehicula. Sed lacinia ligula sit amet
urna tristique commodo. Sed sapien risus, egestas ac tempus vel, pellentesque sed
velit. Duis pulvinar blandit suscipit. Curabitur viverra dignissim est quis ornare.
Nam et lectus purus. Integer sed augue vehicula, volutpat est vel, convallis justo.
Suspendisse a convallis nibh, pulvinar rutrum nisi. Fusce ultrices accumsan mauris
vitae ornare. Cras elementum et ante at tincidunt. Sed luctus scelerisque lobortis.
Sed vel dictum enim. Fusce quis arcu euismod, iaculis mi id, placerat nulla.
Pellentesque porttitor felis elementum justo porttitor auctor.

Aliquam finibus metus commodo sem egestas, non mollis odio pretium. Aenean ex
lectus, rutrum nec laoreet at, posuere sit amet lacus. Nulla eros augue, vehicula et
molestie accumsan, dictum vel odio. In quis risus finibus, pellentesque ipsum
blandit, volutpat diam. Etiam suscipit varius mollis. Proin vel luctus nisi, ac
ornare justo. Integer porttitor quam magna. Donec vitae metus tempor, ultricies
risus in, dictum erat. Integer porttitor faucibus vestibulum. Class aptent taciti
sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Vestibulum
ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae; Nam
semper congue ante, a ultricies velit venenatis vitae. Proin non neque sit amet ex
commodo congue non nec elit. Nullam vel dignissim ipsum. Duis sed lobortis ante.
Aenean feugiat rutrum magna ac luctus.

Ut imperdiet non ante sit amet rutrum. Cras vel massa eget nisl gravida auctor.
Nulla bibendum ut tellus ut rutrum. Quisque malesuada lacinia felis, vitae semper
elit. Praesent sit amet velit imperdiet, lobortis nunc at, faucibus tellus. Nullam
porttitor augue mauris, a dapibus tellus ultricies et. Fusce aliquet nec velit in
mattis. Sed mi ante, lacinia eget ornare vel, faucibus at metus.

Pellentesque nec viverra metus. Sed aliquet pellentesque scelerisque. Duis efficitur
erat sit amet dui maximus egestas. Nullam blandit ante tortor. Suspendisse vitae
consectetur sem, at sollicitudin neque. Suspendisse sodales faucibus eros vitae
pellentesque. Cras non quam dictum, pellentesque urna in, ornare erat. Praesent leo
est, aliquet et euismod non, hendrerit sed urna. Sed convallis porttitor est, vel
aliquet felis cursus ac. Vivamus feugiat eget nisi eu molestie. Phasellus tincidunt
nisl eget molestie consectetur. Phasellus vitae ex ut odio sollicitudin vulputate.
Sed et nulla accumsan, eleifend arcu eget, gravida neque. Donec sit amet tincidunt
eros. Ut in volutpat ante.
";

fn word_in_lorem(c: &mut Criterion) {
    let peg = r#"
        document = ("amet" / .)*;
    "#;
    let content = LOREM.repeat(20);
    let content = content.as_bytes();

    c.bench_function("word_in_lorem_jit", |bench| {
        let parser = Jit::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(|| {
            parser.parse(content).unwrap();
        });
    });

    c.bench_function("word_in_lorem_interpreted", |bench| {
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(|| {
            parser.parse(content).unwrap();
        });
    });
}

fn json(c: &mut Criterion) {
    let peg = include_str!("../pegs/json.peg");
    let content = include_str!("large.json").repeat(20);
    let content = content.as_bytes();

    c.bench_function("json_jit", |bench| {
        let parser = Jit::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(|| {
            parser.parse(content).unwrap();
        });
    });

    c.bench_function("json_interpreted", |bench| {
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(|| {
            parser.parse(content).unwrap();
        });
    });
}

fn rust(c: &mut Criterion) {
    let peg = include_str!("../../../runtime/language/rust/syntax.peg");
    let content = RUST.repeat(20);
    let content = content.as_bytes();

    c.bench_function("rust_jit", |bench| {
        let parser = Jit::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(|| {
            parser.parse(content).unwrap();
        });
    });

    c.bench_function("rust_interpreted", |bench| {
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        bench.iter(|| {
            parser.parse(content).unwrap();
        });
    });
}

criterion_group!(benches, rust, json, word_in_lorem);
criterion_main!(benches);
