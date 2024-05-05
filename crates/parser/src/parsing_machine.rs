mod compiler;
mod op;
mod set;
mod stack;

use std::io;

use crate::{
    grammar,
    parsing_machine::{
        compiler::Compiler,
        stack::{Capture, CaptureList, Stack, StackEntry},
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
    program: Vec<Operation>,
}

impl Parser {
    pub fn new<R: io::Read>(read: R) -> Result<Parser, ParseError> {
        let rules =
            grammar::parse_rules(read).map_err(|err| ParseError::Grammar(err.to_string()))?;
        let compiler = Compiler::new(&rules);
        let program = compiler.compile();
        println!("---- Prgoram ----");
        println!("{:?}", program);

        let parser = Parser {
            program: program.program,
        };
        Ok(parser)
    }
    pub fn parse<B: ByteReader>(&self, reader: B) -> bool {
        let captures = self.parse_impl(reader);

        println!("---- Captures ----");
        println!("{captures:?}");
        captures.is_some()
    }

    fn parse_impl<B: ByteReader>(&self, reader: B) -> Option<CaptureList> {
        use Operation::*;

        let slen = reader.len();
        // Instruction pointer
        let mut ip = 0;
        // Subject pointer
        let mut sp = 0;
        let mut state = State::Normal;
        let mut stack: Stack = Stack::new();
        let mut global_captures = CaptureList::new();

        loop {
            if state == State::Failure {
                loop {
                    match stack.pop() {
                        Some(StackEntry::Backtrack { addr, spos, .. }) => {
                            ip = addr;
                            sp = spos;
                            state = State::Normal;
                            break;
                        }
                        None => return None,
                        _ => {}
                    }
                }
            }

            let op = &self.program[ip];
            println!("ip: {ip}, sp: {sp}, op: {op:?}");

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
                    let sbyte = reader.at(sp);
                    if set[sbyte] {
                        ip += 1;
                        sp += 1;
                    } else {
                        state = State::Failure;
                    }
                }
                Return => match stack.pop().unwrap() {
                    StackEntry::Return {
                        addr, mut captures, ..
                    } => {
                        ip = addr;
                        global_captures.append(&mut captures);
                    }
                    _ => unreachable!("Invalid stack entry pop at return"),
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
                        _ => unreachable!("Invalid stack entry pop at partial commit"),
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
                End => return Some(global_captures),
                EndFail => return None,
                BackCommit(l) => {
                    match stack.pop().unwrap() {
                        StackEntry::Backtrack { spos, .. } => sp = spos,
                        _ => unreachable!("Invalid stack entry pop at back commit"),
                    }
                    ip = *l;
                }
                CaptureBegin(id) => {
                    stack.push_capture(Capture {
                        id: *id,
                        start: sp,
                        len: 0,
                        captures: vec![],
                    });
                    ip += 1;
                }
                CaptureEnd => {
                    match stack.pop().unwrap() {
                        StackEntry::Capture { mut capture } => {
                            println!("Stack top: {:?}", stack.last_mut());
                            capture.len = sp - capture.start;

                            let cap_list = stack
                                .last_mut()
                                .map(StackEntry::captures_mut)
                                .unwrap_or(&mut global_captures);

                            cap_list.push(capture);
                        }
                        _ => unreachable!("Invalid stack entry pop at capture end"),
                    }
                    println!("Stack top after: {:?}", stack.last_mut());
                    ip += 1;
                }
                _ => unreachable!(),
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

        if result {
            println!("accepted");
        } else {
            println!("declined");
        }
    }

    #[test]
    fn parse_simple() {
        let peg = "
            document = _ value _;
            WHITESPACE = [ \\t\\r\\n];
            _ = WHITESPACE*;
            @show
            value = \"abba\";
            ";

        let content = "\r\nabba";

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        println!("Result: {result}");
    }
}
