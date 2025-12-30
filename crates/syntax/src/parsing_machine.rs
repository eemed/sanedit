mod captures;
mod compiler;
mod jit;
mod op;
mod set;
mod stack;

use std::cmp::min;

pub use self::captures::{Capture, CaptureID, CaptureIter, CaptureList, Captures};
pub(crate) use self::compiler::Program;

use anyhow::bail;
use captures::ParserRef;

use crate::{
    grammar::{Annotation, Rules},
    parsing_machine::stack::{Stack, StackEntry},
    source::Source,
    ParseError,
};
pub(crate) use compiler::Compiler;
pub use jit::Jit;

pub(crate) use self::op::{Addr, Operation};

// https://github.com/roberto-ieru/LPeg/blob/master/lpvm.c

#[derive(Debug)]
enum Kind {
    Open,
    Close,
    Backref,
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
            _ => {}
        }
    }

    captures
}

/// Find backreference for capture id in current captures stack.
/// Returns (position, length) pair of the match in the source
fn find_backreference(bref: CaptureID, captures: &[PartialCapture]) -> (u64, u64) {
    let mut nbackrefs = 0;
    let mut stack = vec![];
    for pcap in captures.iter().rev() {
        match pcap.kind {
            Kind::Open => {
                if let Some(end) = stack.pop() {
                    if pcap.id != bref {
                        continue;
                    }

                    if nbackrefs == 0 {
                        return (pcap.pos, end - pcap.pos);
                    } else {
                        nbackrefs -= 1;
                    }
                }
            }
            Kind::Close => stack.push(pcap.pos),
            Kind::Backref => {
                if pcap.id != bref {
                    continue;
                }

                nbackrefs += 1;
            }
        }
    }

    unreachable!()
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Failure,
}

pub(crate) type SubjectPosition = u64;

#[derive(Debug, Clone)]
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
    pub fn captures<'a, 'b, S: Source>(&'a self, reader: &'b mut S) -> CaptureIter<'a, 'b, S> {
        CaptureIter {
            parser: ParserRef::Interpreted(self),
            sp: 0,
            sp_rev: reader.len(),
            source: reader,
        }
    }

    /// Match whole text and return captures, fails if the text does not match
    pub fn parse<S: Source>(&self, reader: &mut S) -> Result<CaptureList, ParseError> {
        self.do_parse(reader, 0)
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
                    'done: loop {
                        let next16 = min(sp + 32, reader.len());
                        if next16 == sp {
                            break;
                        }
                        let Some(slice) = reader.slice(sp..next16) else {
                            break;
                        };
                        for byte in slice {
                            if set.has(*byte) {
                                sp += 1;
                            } else {
                                break 'done;
                            }
                        }
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
                Backreference(r) => {
                    let (br_start, br_len) = find_backreference(*r, &captures);
                    if reader.matches_self(sp, br_start, br_len) {
                        captures.push(PartialCapture {
                            id: *r,
                            kind: Kind::Backref,
                            pos: sp,
                        });
                        captop += 1;
                        ip += 1;
                        sp += br_len;
                    } else {
                        state = State::Failure;
                    }
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

                        if reader.stop() {
                            return Err(ParseError::UserStop.into());
                        }

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
        let mut content = include_bytes!("../benches/large.json");

        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_toml() {
        let peg = include_str!("../pegs/toml.peg");
        let mut content = include_bytes!("../benches/sample.toml");

        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_invalid_json() {
        let peg = include_str!("../pegs/json.peg");
        let mut content = "{ \"hello\": \"world, \"another\": \"line\" }";

        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_simple_1() {
        let peg = "
            name            = (!space !nl .)+;
            space           = [ \\t];
            nl              = \"\\r\\n\" / \"\\r\" / \"\\n\";
            ";

        let mut content = b"abba";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_simple_2() {
        let peg = "
            string          = \"\\\"\" (\"\\\\\" escape_char / [^\"])* \"\\\"\";
            escape_char     = \"0\" / \"t\" / \"n\" / \"r\" / \"\\\"\" / \"\\\\\";
            ";

        let mut content = b"\"registry+https://github.com/rust-lang/crates.io-index\"";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_not_followed() {
        let peg = "
            line_end        = comment? !(!nl .);
            nl              = \"\\r\\n\" / \"\\r\" / \"\\n\";
            comment         = \"#\" (!nl .)*;
            ";

        let mut content = b"# abba\n";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_backrefs() {
        fn text(parser: &ParsingMachine, cap: &Capture, content: &[u8]) -> String {
            let label = parser.label_for(cap.id);
            format!(
                "{label}: {:?}",
                std::str::from_utf8(&content[cap.start as usize..cap.end as usize]).unwrap()
            )
        }

        let peg = r#"
            doc = (tag / ws)*;
            ws = [ \t] / "\n";
            tag = "<" name ">"  (!"</" ws* ( tag / .) )*  ws* "</" bref_name ">";

            @show
            bref_name = @backref(name);

            @show
            name = [a..zA..Z0..9]+;
            "#;

        let mut content = b"<document><body></body></document>";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");

        let caps = result.unwrap();
        assert_eq!(caps.len(), 4);
        assert_eq!(text(&parser, &caps[0], content), r#"name: "document""#);
        assert_eq!(text(&parser, &caps[1], content), r#"name: "body""#);
        assert_eq!(text(&parser, &caps[2], content), r#"bref_name: "body""#);
        assert_eq!(text(&parser, &caps[3], content), r#"bref_name: "document""#);

        let peg = r#"
            doc = (tag / ws)*;
            ws = [ \t] / "\n";
            tag = "<" name ">"  (!"</" ws* ( tag / .) )*  ws* "</" @backref(name) ">";

            @show
            name = [a..zA..Z0..9]+;
            "#;

        let mut content = b"<document><body></body></document>";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(&mut content);
        assert!(result.is_ok(), "Parse failed with {result:?}");

        let caps = result.unwrap();
        assert_eq!(caps.len(), 2);
        assert_eq!(text(&parser, &caps[0], content), r#"name: "document""#);
        assert_eq!(text(&parser, &caps[1], content), r#"name: "body""#);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn parse_any_utf8() {
        let peg = r"any = [\u{0}..\u{10ffff}];";
        let parser = ParsingMachine::from_read(std::io::Cursor::new(peg)).unwrap();

        for i in 0..'\u{10ffff}' as u32 {
            if let Some(ch) = char::from_u32(i) {
                let result = parser.parse(&mut ch.to_string().as_str());
                assert!(result.is_ok(), "Failed to parse char: {ch:?},  {result:?}");
            }
        }
    }
}
