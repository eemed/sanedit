mod compiler;
mod op;
mod set;
mod stack;

pub use self::op::CaptureID;
pub use self::stack::{Capture, CaptureList};

use std::io;

use anyhow::bail;

use crate::{
    grammar,
    parsing_machine::{
        compiler::Compiler,
        stack::{Stack, StackEntry},
    },
    ByteReader, ParseError,
};

use self::op::Operation;

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Failure,
}

#[derive(Debug)]
pub struct Parser {
    labels: Box<[String]>,
    program: Vec<Operation>,
}

impl Parser {
    pub fn new<R: io::Read>(read: R) -> Result<Parser, ParseError> {
        let rules =
            grammar::parse_rules(read).map_err(|err| ParseError::Grammar(err.to_string()))?;
        let compiler = Compiler::new(&rules);
        let program = compiler
            .compile()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;
        log::info!("---- Prgoram ----");
        log::info!("{:?}", program);
        let labels = rules
            .into_iter()
            .map(|rinfo| rinfo.display_name().into())
            .collect();

        let parser = Parser {
            labels,
            program: program.program,
        };
        Ok(parser)
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        &self.labels[id]
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
        let mut state = State::Normal;
        let mut stack: Stack = Stack::new();
        let mut global_captures = CaptureList::new();
        let mut farthest_failure: Option<(usize, usize, CaptureList)> = None;

        loop {
            if state == State::Failure {
                loop {
                    match stack.pop() {
                        Some(StackEntry::Backtrack {
                            addr,
                            spos,
                            captures,
                        }) => {
                            ip = addr;
                            sp = spos;
                            state = State::Normal;
                            break;
                        }
                        Some(StackEntry::Return { captures, addr }) => {
                            match &mut farthest_failure {
                                Some((fsp, a, fcaps)) => {
                                    if captures.is_empty() {
                                        continue;
                                    }

                                    let last_sp =
                                        captures.last().map(|cap| cap.start + cap.len).unwrap();

                                    if last_sp < *fsp {
                                        let mut caps = captures;
                                        caps.append(fcaps);
                                        *fcaps = caps;
                                    } else if sp > *fsp
                                        || sp == *fsp && captures.len() > fcaps.len()
                                    {
                                        farthest_failure = Some((sp, addr, captures));
                                    }
                                }
                                None => farthest_failure = Some((sp, addr, captures)),
                            }
                        }
                        None => match farthest_failure.take() {
                            Some((fsp, a, mut captures)) => {
                                sp = captures
                                    .last()
                                    .map(|cap| cap.start + cap.len)
                                    .unwrap_or(fsp)
                                    + 1;
                                ip = 0;
                                state = State::Normal;

                                global_captures.append(&mut captures);
                                break;
                            }
                            None => {
                                if global_captures.is_empty() {
                                    bail!("No stack entry to backtrack to");
                                } else {
                                    return Ok(global_captures);
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }

            let op = &self.program[ip];
            // println!("ip: {ip}, sp: {sp}, op: {op:?}");

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
                    stack.push(StackEntry::Return {
                        addr: ip + 1,
                        captures: vec![],
                    });
                    ip = *l;
                }
                Commit(l) => {
                    stack.pop_and_prop(&mut global_captures);
                    ip = *l;
                }
                Choice(l) => {
                    stack.push(StackEntry::Backtrack {
                        addr: *l,
                        spos: sp,
                        captures: vec![],
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
                    if sp < slen && set[reader.at(sp)] {
                        ip += 1;
                        sp += 1;
                    } else {
                        state = State::Failure;
                    }
                }
                Return => match stack.pop_and_prop(&mut global_captures) {
                    Some(StackEntry::Return {
                        addr, mut captures, ..
                    }) => {
                        ip = addr;
                        global_captures.append(&mut captures);
                    }
                    e => bail!("Invalid stack entry pop at return: {e:?}"),
                },
                Fail => {
                    state = State::Failure;
                }
                PartialCommit(l) => {
                    let entry = stack.pop_and_prop(&mut global_captures);
                    match entry {
                        Some(StackEntry::Backtrack { addr, .. }) => {
                            stack.push(StackEntry::Backtrack {
                                addr,
                                spos: sp,
                                captures: vec![],
                            })
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
                    while set[sp] {
                        sp += 1;
                    }

                    ip += 1;
                }
                End => return Ok(global_captures),
                EndFail => bail!("Parsing failed"),
                BackCommit(l) => {
                    match stack.pop_and_prop(&mut global_captures) {
                        Some(StackEntry::Backtrack { spos, .. }) => sp = spos,
                        e => bail!("Invalid stack entry pop at back commit: {e:?}"),
                    }
                    ip = *l;
                }
                CaptureBegin(id) => {
                    stack.push_capture(Capture {
                        id: *id,
                        start: sp,
                        len: 0,
                        sub_captures: vec![],
                    });
                    ip += 1;
                }
                CaptureEnd => {
                    match stack.pop() {
                        Some(StackEntry::Capture { mut capture }) => {
                            capture.len = sp - capture.start;

                            let cap_list = stack
                                .last_mut()
                                .map(StackEntry::captures_mut)
                                .unwrap_or(&mut global_captures);

                            cap_list.push(capture);
                        }
                        e => bail!("Invalid stack entry pop at capture end: {e:?}"),
                    }
                    ip += 1;
                }
                _ => bail!("Unsupported operation {op:?}"),
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
