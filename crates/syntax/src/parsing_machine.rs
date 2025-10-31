mod captures;
mod compiler;
mod jit;
mod op;
mod set;
mod stack;

pub use self::captures::{Capture, CaptureID, CaptureIter, CaptureList, Captures};
pub(crate) use self::compiler::Program;

use anyhow::bail;
use captures::ParserRef;

use crate::{
    grammar::{Annotation, Rules},
    parsing_machine::stack::{Stack, StackEntry},
    source::Source,
    ByteSource, ParseError,
};
pub(crate) use compiler::Compiler;
pub use jit::Jit;

pub(crate) use self::op::{Addr, Operation};

// https://github.com/roberto-ieru/LPeg/blob/master/lpvm.c

#[derive(Debug)]
enum Kind {
    Open,
    Close,
}

#[derive(Debug)]
struct PartialCapture {
    id: usize,
    kind: Kind,
    pos: SubjectPosition,
}

fn to_captures(partials: Vec<PartialCapture>) -> Vec<Capture> {
    let mut captures = Vec::with_capacity(partials.len() / 2);
    let mut stack = vec![];
    for cap in partials {
        match cap.kind {
            Kind::Open => {
                stack.push(cap);
            }
            Kind::Close => {
                let start_cap = stack.pop().unwrap();
                let capture = Capture {
                    id: start_cap.id,
                    start: start_cap.pos,
                    end: cap.pos,
                };
                captures.push(capture);
            }
        }
    }

    captures
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Failure,
}

pub(crate) type SubjectPosition = u64;

#[derive(Debug)]
pub struct ParsingMachine {
    rules: Rules,
    program: Program,
}

impl ParsingMachine {
    pub(crate) fn new(rules: Rules, program: Program) -> ParsingMachine {
        ParsingMachine { rules, program }
    }

    pub fn from_read<R: std::io::Read>(read: R) -> Result<ParsingMachine, ParseError> {
        let rules = Rules::parse(read).map_err(|err| ParseError::Grammar(err.to_string()))?;
        Self::from_rules(rules)
    }

    pub(crate) fn from_rules(rules: Rules) -> Result<ParsingMachine, ParseError> {
        if rules.is_empty() {
            return Err(ParseError::NoRules);
        }

        let compiler = Compiler::new(&rules);
        let program = compiler
            .compile()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;
        // log::info!("---- Prgoram ----");
        // log::info!("{:?}", program);

        // println!("---- Prgoram ----");
        // println!("{:?}", program);

        let parser = ParsingMachine { rules, program };
        Ok(parser)
    }

    pub(crate) fn rules(&self) -> &Rules {
        &self.rules
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        if let Some(rule) = self.rules.get(id) {
            return &rule.name;
        }

        // If the capture was not from a rule should be from an embedded
        // operation
        "embed"
    }

    pub fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        self.rules
            .get(id)
            .map(|info| info.annotations.as_slice())
            .unwrap_or(&[])
    }

    /// Try to match text multiple times. Skips errors and yields an element only when part of the text matches
    pub fn captures<'a, B: ByteSource>(&'a self, reader: B) -> CaptureIter<'a, B> {
        CaptureIter {
            parser: ParserRef::Interpreted(self),
            sp: 0,
            sp_rev: reader.len(),
            source: reader,
        }
    }

    /// Match whole text and return captures, fails if the text does not match
    pub fn parse<S: Source>(&self, mut reader: S) -> Result<CaptureList, ParseError> {
        self.do_parse(&mut reader, 0)
            .map(|(caps, _)| caps)
            .map_err(|err| ParseError::Parse(err.to_string()))
    }

    pub(crate) fn do_parse<S: Source>(
        &self,
        reader: &mut S,
        sp: u64,
    ) -> anyhow::Result<(CaptureList, u64)> {
        use Operation::*;

        let slen = reader.len();
        // Instruction pointer
        let mut ip = 0;
        // Subject pointer
        let mut sp = sp;
        // State to indicate failure
        let mut state = State::Normal;
        // Stack for backtracking, choices, returns
        let mut stack: Stack = Stack::new();
        // Parts of text to save
        let mut captures = vec![];
        let mut captop = 0;

        // println!("{:?}", self.program);
        loop {
            let op = &self.program.ops[ip];
            // println!("SP: {sp}, ip: {ip}, op: {op:?}");

            match op {
                Jump(l) => {
                    ip = *l;
                }
                Byte(b) => {
                    // if sp < slen && reader.get(sp) == *b {
                    if reader.get(sp).map(|byte| byte == *b).unwrap_or(false) {
                        ip += 1;
                        sp += 1;
                    } else {
                        state = State::Failure;
                    }
                }
                Call(l) => {
                    stack.push(StackEntry::Return { addr: ip + 1 });
                    ip = *l;
                }
                Commit(l) => {
                    stack.pop();
                    ip = *l;
                }
                Choice(l) => {
                    stack.push(StackEntry::Backtrack {
                        addr: *l,
                        spos: sp,
                        caplevel: captop,
                    });
                    ip += 1;
                }
                Any(n) => {
                    if sp + n <= slen {
                        ip += 1;
                        sp += n;
                    } else {
                        state = State::Failure;
                    }
                }
                UTF8Range(a, b) => match reader.char_between(sp, *a, *b) {
                    Some(n) => {
                        ip += 1;
                        sp += n;
                    }
                    None => {
                        state = State::Failure;
                    }
                },
                Set(set) => {
                    // if sp < slen && set.has(reader.get(sp)) {
                    if reader.get(sp).map(|byte| set.has(byte)).unwrap_or(false) {
                        ip += 1;
                        sp += 1;
                    } else {
                        state = State::Failure;
                    }
                }
                Return => match stack.pop() {
                    Some(StackEntry::Return { addr, .. }) => {
                        ip = addr;
                    }
                    e => bail!("Invalid stack entry pop at return: {e:?}"),
                },
                Fail => {
                    state = State::Failure;
                }
                PartialCommit(l) => {
                    let last = stack
                        .last_mut()
                        .expect("No stack entry to pop at partial commit");

                    match last {
                        StackEntry::Backtrack { spos, caplevel, .. } => {
                            *spos = sp;
                            *caplevel = captop;
                        }
                        e => bail!("Invalid stack entry pop at partial commit: {e:?}"),
                    }

                    ip = *l;
                }
                FailTwice => {
                    stack.pop();
                    state = State::Failure;
                }
                Span(set) => {
                    while sp < slen && set.has(reader.get(sp)) {
                        sp += 1;
                    }

                    ip += 1;
                }
                End => {
                    captures.truncate(captop);
                    return Ok((to_captures(captures), sp));
                }
                BackCommit(l) => {
                    match stack.pop() {
                        Some(StackEntry::Backtrack { spos, caplevel, .. }) => {
                            sp = spos;
                            captop = caplevel;
                            captures.truncate(captop);
                        }
                        e => bail!("Invalid stack entry pop at back commit: {e:?}"),
                    }
                    ip = *l;
                }
                TestByte(byte, l) => {
                    // if sp < slen && reader.get(sp) == *byte {
                    if reader.get(sp).map(|b| b == *byte).unwrap_or(false) {
                        stack.push(StackEntry::Backtrack {
                            addr: *l,
                            spos: sp,
                            caplevel: captop,
                        });

                        ip += 1;
                        sp += 1;
                    } else {
                        ip = *l;
                    }
                }
                TestSet(set, l) => {
                    // if sp < slen && set.has(reader.get(sp)) {
                    if reader.get(sp).map(|byte| set.has(byte)).unwrap_or(false) {
                        stack.push(StackEntry::Backtrack {
                            addr: *l,
                            spos: sp,
                            caplevel: captop,
                        });

                        ip += 1;
                        sp += 1;
                    } else {
                        ip = *l;
                    }
                }
                CaptureBegin(id) => {
                    captures.push(PartialCapture {
                        id: *id,
                        kind: Kind::Open,
                        pos: sp,
                    });
                    captop += 1;
                    ip += 1;
                }
                CaptureEnd => {
                    captures.push(PartialCapture {
                        id: 0,
                        kind: Kind::Close,
                        pos: sp,
                    });
                    captop += 1;
                    ip += 1;
                }
                CaptureLate(id, diff) => {
                    captures.push(PartialCapture {
                        id: *id,
                        kind: Kind::Open,
                        pos: sp - *diff,
                    });
                    captop += 1;
                    ip += 1;
                }
            }

            // Recover from failure state
            while state != State::Normal {
                match stack.pop() {
                    Some(StackEntry::Backtrack {
                        addr,
                        spos,
                        caplevel,
                    }) => {
                        state = State::Normal;
                        ip = addr;
                        sp = spos;
                        captop = caplevel;
                        captures.truncate(captop);
                        break;
                    }

                    None => bail!("No stack entry to backtrack to"),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    impl ParsingMachine {
        pub(crate) fn from_rules_unanchored(rules: Rules) -> Result<ParsingMachine, ParseError> {
            let compiler = Compiler::new(&rules);
            let program = compiler
                .compile_unanchored()
                .map_err(|err| ParseError::Preprocess(err.to_string()))?;

            // log::info!("---- Prgoram unanchor ----");
            // log::info!("{:?}", program);
            let parser = ParsingMachine { rules, program };
            Ok(parser)
        }
    }

    #[test]
    fn parse_large_json() {
        let peg = include_str!("../pegs/json.peg");
        let content = include_bytes!("../benches/large.json");

        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_toml() {
        let peg = include_str!("../pegs/toml.peg");
        let content = include_bytes!("../benches/sample.toml");

        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_invalid_json() {
        let peg = include_str!("../pegs/json.peg");
        let content = "{ \"hello\": \"world, \"another\": \"line\" }";

        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_simple_1() {
        let peg = "
            name            = (!space !nl .)+;
            space           = [ \\t];
            nl              = \"\\r\\n\" / \"\\r\" / \"\\n\";
            ";

        let content = b"abba";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_simple_2() {
        let peg = "
            string          = \"\\\"\" (\"\\\\\" escape_char / [^\"])* \"\\\"\";
            escape_char     = \"0\" / \"t\" / \"n\" / \"r\" / \"\\\"\" / \"\\\\\";
            ";

        let content =
            b"\"registry+https://github.com/rust-lang/crates.io-index\"";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_not_followed() {
        let peg = "
            line_end        = comment? !(!nl .);
            nl              = \"\\r\\n\" / \"\\r\" / \"\\n\";
            comment         = \"#\" (!nl .)*;
            ";

        let content = b"# abba\n";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn parse_any_utf8() {
        let peg = r"any = [\u{0}..\u{10ffff}];";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();

        for i in 0..'\u{10ffff}' as u32 {
            if let Some(ch) = char::from_u32(i) {
                let result = parser.parse(ch.to_string().as_str());
                assert!(result.is_ok(), "Failed to parse char: {ch:?},  {result:?}");
            }
        }
    }
}
