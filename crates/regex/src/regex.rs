mod ast;
mod parser;

use std::ops::Range;

pub(crate) use self::parser::Parser;
pub(crate) use ast::Ast;

use crate::{
    cursor::Cursor,
    vm::{Compiler, Program, VMResult, VM},
};

// TODO: Parse into postfix notation to avoid stack overflows of recursive
// descent parser. Compile postfix notation to instructions.
//
// Implement DFA to run simpler searches faster?
pub struct Regex {
    program: Program,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        let ast = Parser::parse(pattern);
        let program = Compiler::compile(&ast);
        Regex { program }
    }

    /// Find the first match in input
    pub fn find(&self, input: &mut impl Cursor) -> Option<Match> {
        match VM::thompson(&self.program, input) {
            VMResult::Match(m) => Some(m),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Match {
    range: Range<usize>,
    captures: Vec<Match>,
}

impl Match {
    pub(crate) fn from_groups(mut groups: Vec<(usize, usize)>) -> Match {
        let (start, end) = groups.remove(0);
        let captures = groups
            .into_iter()
            .map(|(start, end)| Match {
                range: start..end,
                captures: Vec::new(),
            })
            .collect();

        Match {
            range: start..end,
            captures,
        }
    }

    pub fn start(&self) -> usize {
        self.range.start
    }

    pub fn end(&self) -> usize {
        self.range.end
    }

    pub fn captures(&self) -> &[Match] {
        &self.captures
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
    }
}
