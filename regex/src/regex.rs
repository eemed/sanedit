mod ast;
mod parser;

pub(crate) use self::parser::Parser;
pub(crate) use ast::Ast;

use crate::{
    cursor::Cursor,
    vm::{Compiler, Program, VM},
};

pub struct Regex {
    program: Program,
}

impl Regex {
    pub fn new(regex: &str) -> Regex {
        let ast = Parser::parse(regex);
        let program = Compiler::compile(&ast);
        Regex { program }
    }

    pub fn matches(&self, input: &mut impl Cursor) {
        VM::thompson(&self.program, input);
    }
}
