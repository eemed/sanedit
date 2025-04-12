use thiserror::Error;

use crate::grammar::Rule;
use crate::grammar::RuleInfo;
use crate::grammar::Rules;
use crate::Parser;
use crate::ParseError;

pub struct Regex {
    parser: Parser,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, RegexError> {
        let rules = Self::to_rules(pattern)?;
        let parser = Parser::from_rules(rules)?;
        Ok(Regex { parser })
    }

    fn to_rules(pattern: &str) -> Result<Rules, RegexError> {
        let text = include_str!("../pegs/regex.peg");
        let parser = Parser::new(std::io::Cursor::new(text))?;
        let captures = parser.parse(pattern)?;
        let mut caps = captures.iter().peekable();
        let mut rules: Vec<RuleInfo> = vec![];

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
                        if range.end < inner_range.end {
                            break;
                        }

                        let itext = &pattern[inner_range.start as usize..inner_range.end as usize];
                        let ilabel = parser.label_for(inner.id);

                        println!("i: {ilabel}: {itext:?}");

                        match ilabel {
                            "escaped" | "literal" => {
                                literal.extend_from_slice(itext.as_bytes());
                            }
                            _ => unreachable!(),
                        }

                        caps.next();
                    }

                    if !literal.is_empty() {
                        let name = format!("{}-sequence", rules.len());
                        let rule = Rule::ByteSequence(literal);
                        rules.push(RuleInfo::new_hidden(&name, rule));
                    }
                }
                _ => {}
            }
        }

        // let info = RuleInfo {
        //     top: true,
        //     annotations: vec![],
        //     name: "regex".into(),
        //     rule: Rule::Sequence(rules),
        // };

        let rules = Rules::new(rules.into_boxed_slice());
        Ok(rules)
    }

   pub fn is_match<B: AsRef<[u8]>>(&self, bytes: &B) -> bool {
        let bytes = bytes.as_ref();
        match self.parser.parse(bytes) {
            Ok(_) => true,
            Err(_e) => false,
        }
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
        assert!(!regex.is_match(b"abdc"));
        assert!(regex.is_match(b"abc."));
    }
}
