mod compiler;
mod op;
mod set;
mod stack;

use self::compiler::Program;
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

use self::op::{Addr, Operation};

#[derive(Debug)]
struct FarthestFailure {
    sp: usize,
    fails: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Failure,
}

#[derive(Debug)]
pub struct Parser {
    labels: Box<[String]>,
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
        log::info!("---- Prgoram ----");
        log::info!("{:?}", program);

        // println!("---- Prgoram ----");
        // println!("{:?}", program);

        let labels = rules
            .into_iter()
            .map(|rinfo| rinfo.display_name().into())
            .collect();

        let parser = Parser { labels, program };
        Ok(parser)
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        &self.labels[id]
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
        let mut state = State::Normal;
        let mut stack: Stack = Stack::new();
        let mut global_captures = CaptureList::new();
        let mut farthest_failure: Option<FarthestFailure> = None;

        loop {
            while state != State::Normal {
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

                    None => {
                        if global_captures.is_empty() {
                            bail!("No stack entry to backtrack to");
                        } else {
                            return Ok(global_captures);
                        }
                    }
                    _ => {}
                }
            }

            let op = &self.program.ops[ip];
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
                Checkpoint => {
                    // println!("Checkpoint: {}: sp: {sp}", self.label_for_op(ip));
                    // stack.print();

                    // for e in stack.iter() {
                    //     match e {
                    //         StackEntry::Backtrack { addr, spos, captures } => {
                    //             if !captures.is_empty() {
                    // println!("Checkpoint bt noempty: {}: sp: {sp}", self.label_for_op(ip));
                    //             }
                    //         }
                    //         _ => {}
                    //         // StackEntry::Capture { capture } => todo!(),
                    //         // StackEntry::Return { addr, captures } => todo!(),
                    //     }
                    // }

                    println!("");
                    println!("+ Before sp: {sp}");
                    stack.print();
                    // stack.checkpoint(sp);
                    // println!("");
                    // println!("- After sp: {sp}");
                    // stack.print();

                    // We are on the correct parse. No need for stack backtrack
                    // entries anymore
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
    fn parse_invalid_json() {
        let peg = include_str!("../pegs/json.peg");
        let content = "{ \"hello\": \"world\", \"another\": \"line }";

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
