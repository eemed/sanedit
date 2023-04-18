pub(crate) mod parser;
mod error;

use std::ops::Range;

use self::parser::regex_to_postfix;

use crate::{
    cursor::Cursor,
    vm::{Program, VMResult, VM},
};

// Implement DFA to run simpler searches faster?
pub struct Regex {
    program: Program,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        let postfix = regex_to_postfix(pattern);
        let program = Program::try_from(postfix).unwrap();
        Regex { program }
    }

    /// Find the first match in input
    pub fn find(&self, input: &mut impl Cursor) -> Option<Match> {
        match VM::thompson(&self.program, input, true) {
            VMResult::Match(m) => Some(m),
            _ => None,
        }
    }

    pub fn find_all(&self, input: &mut impl Cursor) -> Option<Vec<Match>> {
        match VM::thompson(&self.program, input, false) {
            VMResult::All(matches) => Some(matches),
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

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
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
