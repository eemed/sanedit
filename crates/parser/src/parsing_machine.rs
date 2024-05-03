mod compiler;
mod op;
mod set;

use std::io;

use crate::{grammar, parsing_machine::compiler::Compiler};

struct Parser {}

impl Parser {
    pub fn new<R: io::Read>(read: R) -> Parser {
        let rules = grammar::parse_rules(read).unwrap();
        let mut compiler = Compiler::new(&rules);
        compiler.compile();
        todo!()
    }

    pub fn parse(&self) {}
}
