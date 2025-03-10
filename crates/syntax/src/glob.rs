use thiserror::Error;

use crate::{
    grammar::{Rule, RuleInfo, Rules},
    ParseError, Parser,
};

#[derive(Error, Debug)]
pub enum GlobError {
    #[error("Failed to parse grammar: {0}")]
    Parsing(#[from] ParseError),
}

// https://en.wikipedia.org/wiki/Glob_(programming)
//
#[allow(dead_code)]
#[derive(Debug)]
pub struct Glob {
    parser: Parser,
}

#[allow(dead_code)]
impl Glob {
    pub fn new(pattern: &str) -> Result<Glob, GlobError> {
        // Just testing here that this works OK, should probably do something better => just parse manually as this is prett simple
        let text = include_str!("../pegs/glob.peg");
        let parser = Parser::new(std::io::Cursor::new(text))?;
        let captures = parser.parse(pattern)?;
        let mut rules: Vec<Rule> = vec![];
        let mut many = false;

        for cap in captures {
            let label = parser.label_for(cap.id());
            let rule = match label {
                "text" => {
                    let range = cap.range();
                    let text = &pattern[range.start as usize..range.end as usize];
                    Some(Rule::ByteSequence(text.as_bytes().to_vec()))
                }
                "many" => {
                    many = true;
                    None
                }
                _ => None,
            };

            if let Some(rule) = rule {
                if many {
                    many = false;
                    // (!text .)* text
                    rules.push(Rule::Sequence(vec![
                        Rule::ZeroOrMore(Box::new(Rule::Sequence(vec![
                            Rule::NotFollowedBy(Box::new(rule.clone())),
                            Rule::ByteAny,
                        ]))),
                        rule,
                    ]));
                } else {
                    rules.push(rule);
                }
            }
        }

        let info = RuleInfo {
            top: true,
            annotations: vec![],
            name: "glob".into(),
            rule: Rule::Sequence(rules),
        };
        let rules = Rules::new(Box::new([info]));
        let pparse = Parser::from_rules(rules)?;
        Ok(Glob { parser: pparse })
    }

    pub fn is_match<B: AsRef<[u8]>>(&self, bytes: &B) -> bool {
        let bytes = bytes.as_ref();
        match self.parser.parse(bytes) {
            Ok(_) => true,
            Err(_e) => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rust() {
        let glob = Glob::new("*.rs").unwrap();
        assert_eq!(glob.is_match(b".hidden"), false);
        assert_eq!(glob.is_match(b"path/to/glob.rs"), true);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn wildcard() {
        let glob = Glob::new("*aw*").unwrap();
        assert_eq!(glob.is_match(b"lawyer"), true);
        assert_eq!(glob.is_match(b"the law"), true);
        assert_eq!(glob.is_match(b"the lew"), false);
    }

    #[test]
    fn hidden() {
        let glob = Glob::new(".*").unwrap();
        assert_eq!(glob.is_match(b".hidden"), true);
        assert_eq!(glob.is_match(b"path/to/glob.rs"), false);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn question() {
        let glob = Glob::new("?at").unwrap();
        assert_eq!(glob.is_match(b"Cat"), true);
        assert_eq!(glob.is_match(b"Bat"), true);
        assert_eq!(glob.is_match(b"ccat"), false);
    }

    #[test]
    fn alt() {
        let glob = Glob::new("[CB]at").unwrap();
        assert_eq!(glob.is_match(b"Cat"), true);
        assert_eq!(glob.is_match(b"Bat"), true);
        assert_eq!(glob.is_match(b"ccat"), false);
    }

    #[test]
    fn alt_range() {
        let glob = Glob::new("Letter[0-9]").unwrap();
        assert_eq!(glob.is_match(b"Letter8"), true);
        assert_eq!(glob.is_match(b"Letter0"), true);
        assert_eq!(glob.is_match(b"Letter10"), false);
        assert_eq!(glob.is_match(b"Letter"), true);
    }
}
