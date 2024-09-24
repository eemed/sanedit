mod captures;
mod compiler;
mod jit;
mod op;
mod set;
mod stack;

pub use self::captures::{Capture, CaptureID, CaptureList};
use self::compiler::Program;

use std::io;

use anyhow::bail;

use crate::{
    grammar::{self, Annotation, Rules},
    parsing_machine::{
        compiler::Compiler,
        stack::{Stack, StackEntry},
    },
    ByteReader, ParseError,
};

use self::op::{Addr, Operation};

// https://github.com/roberto-ieru/LPeg/blob/master/lpvm.c

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Failure,
}

pub(crate) type SubjectPosition = u64;

#[derive(Debug)]
pub struct Parser {
    rules: Rules,
    program: Program,
}

impl Parser {
    pub fn new<R: io::Read>(read: R) -> Result<Parser, ParseError> {
        let rules =
            grammar::parse_rules(read).map_err(|err| ParseError::Grammar(err.to_string()))?;
        let compiler = Compiler::new(&rules);
        let program = compiler
            .compile()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;
        // log::info!("---- Prgoram ----");
        // log::info!("{:?}", program);

        // println!("---- Prgoram ----");
        // println!("{:?}", program);

        let parser = Parser { rules, program };
        Ok(parser)
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        &self.rules[id].display_name()
    }

    pub fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        &self.rules[id].annotations
    }

    pub fn label_for_op(&self, op: Addr) -> &str {
        self.program
            .names
            .range(..=op)
            .next_back()
            .expect("No name for op index {addr}")
            .1
    }

    pub fn parse<B: ByteReader>(&self, reader: B) -> Result<CaptureList, ParseError> {
        self.do_parse(reader)
            .map_err(|err| ParseError::Parse(err.to_string()))
    }

    fn do_parse<B: ByteReader>(&self, reader: B) -> anyhow::Result<CaptureList> {
        use Operation::*;

        let slen = reader.len();
        // Instruction pointer
        let mut ip = 0;
        // Subject pointer
        let mut sp = 0;
        // State to indicate failure
        let mut state = State::Normal;
        // Stack for backtracking, choices, returns
        let mut stack: Stack = Stack::new();
        // Parts of text to save
        let mut captures = CaptureList::new();
        let mut captop = 0;

        loop {
            let op = &self.program.ops[ip];

            match op {
                Jump(l) => {
                    ip = *l;
                }
                Byte(b) => {
                    if sp < slen && reader.at(sp) == *b {
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
                    if sp + n < slen {
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
                    if sp < slen && set.has(reader.at(sp)) {
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
                        StackEntry::Backtrack {
                            addr,
                            spos,
                            caplevel,
                        } => {
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
                    while set.has(reader.at(sp)) {
                        sp += 1;
                    }

                    ip += 1;
                }
                End => return Ok(captures),
                EndFail => bail!("Parsing failed"),
                BackCommit(l) => {
                    match stack.pop() {
                        Some(StackEntry::Backtrack {
                            addr,
                            spos,
                            caplevel,
                        }) => {
                            sp = spos;
                            captop = caplevel;
                        }
                        e => bail!("Invalid stack entry pop at back commit: {e:?}"),
                    }
                    ip = *l;
                }
                CaptureBegin(id) => {
                    captures.push(Capture::new(*id, sp));
                    captop += 1;
                    ip += 1;
                }
                CaptureEnd => {
                    // Find last unclosed capture
                    let caps = &mut captures[..captop];
                    for i in (0..caps.len()).rev() {
                        let cap = &mut caps[i];
                        if !cap.is_closed() {
                            cap.close(sp);
                            break;
                        }
                    }
                    ip += 1;
                }
                _ => bail!("Unsupported operation {op:?}"),
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

                    None => {
                        if captures.is_empty() {
                            bail!("No stack entry to backtrack to");
                        } else {
                            return Ok(captures);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_large_json() {
        let peg = include_str!("../pegs/json.peg");
        let content = include_str!("../benches/large.json");

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        println!("Result: {result:?}");
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_toml() {
        let peg = include_str!("../pegs/toml.peg");
        let content = include_str!("../benches/sample.toml");

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_invalid_json() {
        let peg = include_str!("../pegs/json.peg");
        let content = "{ \"hello\": \"world, \"another\": \"line\" }";

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
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

        let content = "abba";

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }

    #[test]
    fn parse_simple_2() {
        let peg = "
            string          = \"\\\"\" (\"\\\\\" escape_char / [^\"])* \"\\\"\";
            escape_char     = \"0\" / \"t\" / \"n\" / \"r\" / \"\\\"\" / \"\\\\\";
            ";

        println!("{peg}");
        let content = "\"registry+https://github.com/rust-lang/crates.io-index\"";

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
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

        let content = "# abba\n";

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        assert!(result.is_ok(), "Parse failed with {result:?}");
    }
}
