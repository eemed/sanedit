use thiserror::Error;

use crate::grammar::Rule;
use crate::grammar::RuleInfo;
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
        let mut caps = captures.iter().peekable();
        let mut rules = vec![];

        while caps.peek().is_some() {
            let cap = caps.next().unwrap();
            let range = cap.range();
            let text = &pattern[range.start as usize..range.end as usize];
            let label = parser.label_for(cap.id);

            println!("{label}: {text:?}");

            match label {
                "sequence" => {
                    let mut literal = vec![];
                    while let Some(inner) = caps.peek() {
                        let inner_range = inner.range();
                        if range.end >= inner_range.start {
                            break;
                        }

                        let itext = &pattern[inner_range.start as usize..inner_range.end as usize];
                        let ilabel = parser.label_for(cap.id);

                        match ilabel {
                            "escaped" | "literal" => {
                                literal.extend_from_slice(itext.as_bytes());
                            }
                            _ => unreachable!(),
                        }

                        caps.next();
                    }

                    rules.push(Rule::ByteSequence(literal));
                }
                _ => {}
            }
        }

        let info = RuleInfo {
            top: true,
            annotations: vec![],
            name: "regex".into(),
            rule: Rule::Sequence(rules),
        };
        let rules = Rules::new(Box::new([info]));
        Ok(rules)
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
