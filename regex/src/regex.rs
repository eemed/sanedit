mod ast;
mod parser;

pub(crate) use self::parser::Parser;
pub(crate) use ast::Ast;

use crate::{
    cursor::Cursor,
    vm::{Compiler, Program, VMResult, VM},
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

    pub fn find(&self, input: &mut impl Cursor) -> bool {
        let result = VM::thompson(&self.program, input);
        println!("Result: {result:?}");
        matches!(result, VMResult::Match { .. })
    }
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
