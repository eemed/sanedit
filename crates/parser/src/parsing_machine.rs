mod compiler;
mod op;
mod set;

use std::io;

use crate::{
    grammar,
    parsing_machine::{compiler::Compiler, op::Addr},
    ByteReader, ParseError,
};

use self::op::Operation;

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Failure,
}

enum StackEntry {
    Return(Addr),
    Backtrack(Addr, usize),
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
        let parser = Parser {
            program: program.program,
        };
        Ok(parser)
    }

    pub fn parse<B: ByteReader>(&self, reader: B) -> bool {
        use Operation::*;

        let slen = reader.len();
        // Instruction pointer
        let mut ip = 0;
        // Subject pointer
        let mut sp = 0;
        let mut state = State::Normal;
        let mut stack: Vec<StackEntry> = vec![];

        loop {
            if state == State::Failure {
                loop {
                    match stack.pop() {
                        Some(StackEntry::Backtrack(i, s)) => {
                            ip = i;
                            sp = s;
                            state = State::Normal;
                            break;
                        }
                        None => return false,
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
                    stack.push(StackEntry::Return(ip + 1));
                    ip = *l;
                }
                Commit(l) => {
                    stack.pop();
                    ip = *l;
                }
                Choice(l) => {
                    stack.push(StackEntry::Backtrack(*l, sp));
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
                    StackEntry::Return(l) => ip = l,
                    _ => unreachable!("Tried to pop backtrack entry at return"),
                },
                Fail => {
                    state = State::Failure;
                }
                PartialCommit(l) => {
                    stack.last_mut().map(|entry| match entry {
                        StackEntry::Backtrack(_, s) => *s = sp,
                        _ => unreachable!("Tried to update return entry at partial commit"),
                    });
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
                End => return sp == slen,
                EndFail => return false,
                BackCommit(l) => {
                    match stack.pop().unwrap() {
                        StackEntry::Backtrack(_, s) => sp = s,
                        _ => unreachable!("Tried to pop return entry at back commit"),
                    }
                    ip = *l;
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
    fn parse_whitespace_any() {
        let peg = "
            document = _ value _;
            WHITESPACE = [ \\t\\r\\n];
            _ = WHITESPACE*;
            value = \"abba\";
            ";

        let content = "\r\nbba";

        let parser = Parser::new(std::io::Cursor::new(peg)).unwrap();
        let result = parser.parse(content);
        println!("Result: {result}");
    }
}
