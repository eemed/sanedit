use std::sync::{Arc, OnceLock};

use sanedit_utils::{ranges::OverlappingRanges, sorted_vec::SortedVec};
use thiserror::Error;

use crate::{
    grammar::{Rule, RuleInfo, Rules}, Capture, ParseError, Parser, ParsingMachine
};

#[derive(Error, Debug)]
pub enum GlobError {
    #[error("Failed to parse grammar: {0}")]
    Parsing(#[from] ParseError),
}

pub struct GlobRules(Rules);

impl std::fmt::Display for GlobRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for GlobRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

fn glob_parser() -> &'static ParsingMachine {
    static PARSER: OnceLock<Arc<ParsingMachine>> = OnceLock::new();
    let parser = PARSER.get_or_init(|| {
        let text = include_str!("../pegs/glob.peg");
        let parser = ParsingMachine::new(std::io::Cursor::new(text)).unwrap();
        Arc::new(parser)
    });
    parser.as_ref()
}

/// Done like https://en.wikipedia.org/wiki/Glob_(programming)
///
/// No extglob
#[allow(dead_code)]
#[derive(Debug)]
pub struct Glob {
    parser: Parser,
}

#[allow(dead_code)]
impl Glob {
    pub fn new(pattern: &str) -> Result<Glob, GlobError> {
        let rules = Self::to_rules(pattern)?;
        let parser = Parser::from_rules(rules)?;
        Ok(Glob { parser })
    }

    pub fn parse_pattern(pattern: &str) -> Result<GlobRules, GlobError> {
        let rules = Self::to_rules(pattern)?;
        Ok(GlobRules(rules))
    }

    pub fn from_rules(rules: GlobRules) -> Result<Glob, GlobError> {
        let parser = Parser::from_rules(rules.0)?;
        Ok(Glob { parser })
    }

    fn to_rules(pattern: &str) -> Result<Rules, GlobError> {
        let to_bytes = |cap: &Capture| {
            let range = cap.range();
            pattern[range.start as usize..range.end as usize]
                .as_bytes()
                .to_vec()
        };

        let parser = glob_parser();
        let captures: SortedVec<Capture> = parser.parse(pattern)?.into();
        let mut rules: Vec<RuleInfo> = vec![];
        let mut seq: Vec<Rule> = vec![];

        let mut iter = captures.iter().peekable();
        while let Some(cap) = iter.next() {
            let label = parser.label_for(cap.id());
            match label {
                "negative_brackets" => {
                    let inside = {
                        let mut inside = vec![];
                        while let Some(ncap) = iter.peek() {
                            if ncap.end <= cap.end {
                                inside.push((*ncap).clone());
                                iter.next();
                            } else {
                                break;
                            }
                        }

                        inside
                    };

                    let mut ranges = OverlappingRanges::new();
                    let mut choices = vec![];
                    let mut iiter = inside.iter().peekable();
                    while let Some(ncap) = iiter.next() {
                        let nlabel = parser.label_for(ncap.id());
                        match nlabel {
                            "range" => {
                                // Next 2 should be chars
                                let a = iiter.next().expect("No range a");
                                let b = iiter.next().expect("No range b");
                                ranges.add(to_bytes(a)[0] as u32..to_bytes(b)[0] as u32 + 1);
                            }
                            "char" => {
                                let a = to_bytes(ncap)[0] as u32;
                                ranges.add(a..a + 1);
                            }
                            _ => unreachable!("Invalid label in brackets"),
                        }
                    }

                    ranges.invert(u8::MIN as u32..u8::MAX as u32 + 1);
                    for range in ranges.iter() {
                        choices.push(Rule::ByteRange(range.start as u8, (range.end - 1) as u8));
                    }

                    if choices.len() == 1 {
                        seq.push(choices.pop().unwrap());
                    } else {
                        seq.push(Rule::Choice(choices));
                    }
                }
                "brackets" => {
                    let inside = {
                        let mut inside = vec![];
                        while let Some(ncap) = iter.peek() {
                            if ncap.end <= cap.end {
                                inside.push((*ncap).clone());
                                iter.next();
                            } else {
                                break;
                            }
                        }

                        inside
                    };

                    let mut choices = vec![];
                    let mut iiter = inside.iter().peekable();
                    while let Some(ncap) = iiter.next() {
                        let nlabel = parser.label_for(ncap.id());
                        match nlabel {
                            "range" => {
                                // Next 2 should be chars
                                let a = iiter.next().expect("No range a");
                                let b = iiter.next().expect("No range b");
                                choices.push(Rule::ByteRange(to_bytes(a)[0], to_bytes(b)[0]))
                            }
                            "char" => choices.push(Rule::ByteSequence(to_bytes(ncap))),
                            _ => unreachable!("Invalid label in brackets"),
                        }
                    }
                    if choices.len() == 1 {
                        seq.push(choices.pop().unwrap());
                    } else {
                        seq.push(Rule::Choice(choices));
                    }
                }
                "text" => seq.push(Rule::ByteSequence(to_bytes(cap))),
                "wildcard" => {
                    let prev_i = rules.len();
                    let wildcard_i = prev_i + 1;
                    let next_i = prev_i + 2;

                    let wildcard = Rule::Ref(wildcard_i);
                    seq.push(wildcard.clone());

                    let prev = RuleInfo {
                        top: false,
                        annotations: vec![],
                        name: format!("rule-{prev_i}"),
                        rule: Rule::Sequence(std::mem::take(&mut seq)),
                    };
                    rules.push(prev);

                    // prev = ... Ref(wildcard)
                    // wildcard = Ref(next) / [^/] Ref(wildcard)

                    let next = Rule::Ref(next_i);
                    let rule = Rule::Choice(vec![
                        next,
                        Rule::Sequence(vec![
                            Rule::Choice(vec![
                                Rule::ByteRange(u8::MIN, '/' as u8 - 1),
                                Rule::ByteRange('/' as u8 + 1, u8::MAX),
                            ]),
                            wildcard,
                        ]),
                    ]);

                    let wcard = RuleInfo {
                        top: false,
                        annotations: vec![],
                        name: format!("wildcard-{wildcard_i}"),
                        rule,
                    };
                    rules.push(wcard);
                }
                "recursive_wildcard" => {
                    let prev_i = rules.len();
                    let wildcard_i = prev_i + 1;
                    let next_i = prev_i + 2;

                    let wildcard = Rule::Ref(wildcard_i);
                    seq.push(wildcard.clone());

                    let prev = RuleInfo {
                        top: false,
                        annotations: vec![],
                        name: format!("rule-{prev_i}"),
                        rule: Rule::Sequence(std::mem::take(&mut seq)),
                    };
                    rules.push(prev);

                    // prev = ... Ref(wildcard)
                    // wildcard = Ref(next) / "/"? [^/]+ Ref(wildcard)

                    let next = Rule::Ref(next_i);
                    let rule = Rule::Choice(vec![
                        next,
                        Rule::Sequence(vec![
                            Rule::Optional(Rule::ByteSequence("/".into()).into()),
                            Rule::OneOrMore(
                                Rule::Choice(vec![
                                    Rule::ByteRange(u8::MIN, '/' as u8 - 1),
                                    Rule::ByteRange('/' as u8 + 1, u8::MAX),
                                ])
                                .into(),
                            ),
                            wildcard,
                        ]),
                    ]);

                    let wcard = RuleInfo {
                        top: false,
                        annotations: vec![],
                        name: format!("wildcard-{wildcard_i}"),
                        rule,
                    };
                    rules.push(wcard);
                }
                "any" => seq.push(Rule::ByteAny),
                "separator" => seq.push(Rule::ByteSequence("/".into())),
                _ => {}
            }
        }

        // Assert end
        seq.push(Rule::NotFollowedBy(Rule::ByteAny.into()));

        let info = RuleInfo {
            top: false,
            annotations: vec![],
            name: "final".into(),
            rule: Rule::Sequence(seq),
        };
        rules.push(info);
        rules[0].top = true;

        let rules = Rules::new(rules.into());
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn glob_rust() {
        let glob = Glob::new("**/*.rs").unwrap();
        assert_eq!(glob.is_match(b".hidden"), false);
        assert_eq!(glob.is_match(b"path/to/glob.rs"), true);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn glob_wildcard() {
        let glob = Glob::new("*aw*").unwrap();
        assert_eq!(glob.is_match(b"lawyer"), true);
        assert_eq!(glob.is_match(b"the law"), true);
        assert_eq!(glob.is_match(b"the lew"), false);
        assert_eq!(glob.is_match(b"xxxxxxxxxawxxxxxxxx"), true);
        assert_eq!(glob.is_match(b"xxxxxxxxxxxxxxxxx"), false);
    }

    #[test]
    fn glob_hidden() {
        let glob = Glob::new(".*").unwrap();
        assert_eq!(glob.is_match(b".hidden"), true);
        assert_eq!(glob.is_match(b"path/to/glob.rs"), false);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn glob_question() {
        let glob = Glob::new("?at").unwrap();
        assert_eq!(glob.is_match(b"Cat"), true);
        assert_eq!(glob.is_match(b"Bat"), true);
        assert_eq!(glob.is_match(b"ccat"), false);
    }

    #[test]
    fn glob_alt_1() {
        let glob = Glob::new("[CB]at").unwrap();
        assert_eq!(glob.is_match(b"Cat"), true);
        assert_eq!(glob.is_match(b"Bat"), true);
        assert_eq!(glob.is_match(b"ccat"), false);
    }

    #[test]
    fn glob_alt_range() {
        let glob = Glob::new("Letter[0-9]").unwrap();
        assert_eq!(glob.is_match(b"Letter8"), true);
        assert_eq!(glob.is_match(b"Letter0"), true);
        assert_eq!(glob.is_match(b"Letter10"), false);
        assert_eq!(glob.is_match(b"Letter"), false);
    }
}
