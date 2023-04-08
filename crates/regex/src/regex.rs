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
    pub fn new(pattern: &str) -> Regex {
        let ast = Parser::parse(pattern);
        let program = Compiler::compile(&ast);
        Regex { program }
    }

    pub fn new_literal(literal: &str) -> Regex {
        todo!()
    }

    pub fn find(&self, input: &mut impl Cursor) -> RegexResult {
        VM::thompson(&self.program, input)
    }
}

#[derive(Debug)]
pub enum RegexResult {
    /// The first pair is the whole match, and the rest are capturing groups
    /// used in the regex.
    Match(Vec<(usize, usize)>),
    NoMatch,
}

#[cfg(test)]
mod test {
    use crate::cursor::StringCursor;

    use super::*;

    #[test]
    fn simple() {
        let mut text: StringCursor = "ca".into();
        let regex = Regex::new("car?");
        println!("{:?}", regex.program);
        let matched = regex.find(&mut text);
        assert!(matched);
    }
}
