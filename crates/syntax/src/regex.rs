use thiserror::Error;

use crate::grammar::Rules;
use crate::Parser;
use crate::ParseError;

pub struct Regex {
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, RegexError> {
        let rules = Self::to_rules(pattern)?;
        todo!()
    }

    fn to_rules(pattern: &str) -> Result<Rules, RegexError> {
        let text = include_str!("../pegs/regex.peg");
        let parser = Parser::new(std::io::Cursor::new(text))?;
        let captures = parser.parse(pattern)?;

        for cap in captures {
            let range = cap.range();
            let text = &pattern[range.start as usize..range.end as usize];
            let name = parser.label_for(cap.id);
            println!("{name}: {text:?}");
        }

        todo!()
    }
}

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("Failed to parse grammar: {0}")]
    Parsing(#[from] ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_literal() {
        let regex = Regex::new("abc").unwrap();
    }

    #[test]
    fn regex_zero_or_more() {
        let regex = Regex::new("a*").unwrap();
    }

    #[test]
    fn regex_escaped() {
        let regex = Regex::new("abc\\.").unwrap();
    }
}