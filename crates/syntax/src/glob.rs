use std::sync::{Arc, OnceLock};

use sanedit_utils::{ranges::OverlappingRanges, sorted_vec::SortedVec};
use thiserror::Error;

use crate::{
    grammar::{Rule, RuleInfo, Rules},
    Capture, ParseError, ParserKind as Parser,
};

#[derive(Error, Debug)]
pub enum GlobError {
    #[error("Failed to parse grammar: {0}")]
    Parsing(#[from] ParseError),
}

pub struct GlobRules(Rules, GlobOptions);

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

fn glob_parser() -> &'static Parser {
    static PARSER: OnceLock<Arc<Parser>> = OnceLock::new();
    let parser = PARSER.get_or_init(|| {
        let text = include_str!("../pegs/glob.peg");
        let parser = Parser::new(std::io::Cursor::new(text)).unwrap();
        Arc::new(parser)
    });
    parser.as_ref()
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobOptions {
    pub negated: bool,
    pub directory_only: bool,
}

/// Git ignore globs
/// https://git-scm.com/docs/gitignore
#[allow(dead_code)]
#[derive(Debug)]
pub struct GitGlob {
    parser: Parser,
    options: GlobOptions,
}

#[allow(dead_code)]
impl GitGlob {
    pub fn new(pattern: &str) -> Result<GitGlob, GlobError> {
        let (rules, options) = Self::to_rules(pattern)?;
        let parser = Parser::from_rules(rules)?;
        Ok(GitGlob { parser, options })
    }

    pub fn parse_pattern(pattern: &str) -> Result<GlobRules, GlobError> {
        let (rules, options) = Self::to_rules(pattern)?;
        Ok(GlobRules(rules, options))
    }

    pub fn options(&self) -> &GlobOptions {
        &self.options
    }

    pub fn from_rules(rules: GlobRules) -> Result<GitGlob, GlobError> {
        let parser = Parser::from_rules(rules.0)?;
        Ok(GitGlob {
            parser,
            options: rules.1,
        })
    }

    fn to_rules(mut pattern: &str) -> Result<(Rules, GlobOptions), GlobError> {
        let to_bytes = |cap: &Capture| {
            let range = cap.range();
            pattern.as_bytes()[range.start as usize..range.end as usize].to_vec()
        };

        let parser = glob_parser();
        let captures: SortedVec<Capture> = parser.parse(&mut pattern)?.into();
        let mut rules: Vec<RuleInfo> = vec![];
        let mut seq: Vec<Rule> = vec![];
        let mut negated = false;
        let mut separator_at_beginning_or_middle = false;
        let mut last_label = None;
        let mut iter = captures.iter().peekable();

        while let Some(cap) = iter.next() {
            if let Some(ll) = last_label {
                if ll == "separator" {
                    separator_at_beginning_or_middle = true;
                }
            }

            let label = parser.label_for(cap.id());
            match label {
                "negate" => negated = true,
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
                "escape_char" | "text" => seq.push(Rule::ByteSequence(to_bytes(cap))),
                "wildcard" => {
                    let prev_i = rules.len();
                    let wildcard_i = prev_i + 1;
                    let next_i = prev_i + 2;

                    let wildcard = Rule::Ref(wildcard_i);
                    seq.push(wildcard.clone());

                    let prev = RuleInfo::new(
                        format!("rule-{prev_i}"),
                        Rule::Sequence(std::mem::take(&mut seq)),
                    );
                    rules.push(prev);

                    // prev = ... Ref(wildcard)
                    // wildcard = Ref(next) / [^/] Ref(wildcard)

                    let next = Rule::Ref(next_i);
                    let rule = Rule::Choice(vec![
                        next,
                        Rule::Sequence(vec![
                            Rule::Choice(vec![
                                Rule::ByteRange(u8::MIN, b'/' - 1),
                                Rule::ByteRange(b'/' + 1, u8::MAX),
                            ]),
                            wildcard,
                        ]),
                    ]);

                    let wcard = RuleInfo::new(format!("wildcard-{wildcard_i}"), rule);
                    rules.push(wcard);
                }
                "recursive_wildcard" => {
                    let prev_i = rules.len();
                    let wildcard_i = prev_i + 1;
                    let next_i = prev_i + 2;

                    let wildcard = Rule::Ref(wildcard_i);
                    seq.push(wildcard.clone());

                    let prev = RuleInfo::new(
                        format!("rule-{prev_i}"),
                        Rule::Sequence(std::mem::take(&mut seq)),
                    );
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
                                    Rule::ByteRange(u8::MIN, b'/' - 1),
                                    Rule::ByteRange(b'/' + 1, u8::MAX),
                                ])
                                .into(),
                            ),
                            wildcard,
                        ]),
                    ]);

                    let wcard = RuleInfo::new(format!("wildcard-{wildcard_i}"), rule);
                    rules.push(wcard);
                }
                "any" => {
                    seq.push(Rule::Choice(vec![
                        Rule::ByteRange(u8::MIN, b'/' - 1),
                        Rule::ByteRange(b'/' + 1, u8::MAX),
                    ]));
                }
                "separator" => {
                    // Recursive wildcard a/**/b matches also a/b, we need to possibly eat the /
                    if last_label == Some("recursive_wildcard") {
                        seq.push(Rule::Optional(Rule::ByteSequence("/".into()).into()));
                    } else if seq.is_empty() {
                        // Leading slash is optional
                        seq.push(Rule::Optional(Rule::ByteSequence("/".into()).into()));
                    } else {
                        seq.push(Rule::ByteSequence("/".into()));
                    }
                }
                _ => {}
            }

            last_label = Some(label);
        }

        let directory_only = last_label == Some("separator");
        if directory_only {
            seq.pop();
        }

        seq.push(Rule::Optional(Rule::ByteSequence("/".into()).into()));
        seq.push(Rule::NotFollowedBy(Rule::ByteAny.into()));

        let info = RuleInfo::new("final".into(), Rule::Sequence(seq));
        rules.push(info);

        if !separator_at_beginning_or_middle {
            // Should match at any level => add recursive wildcard to start
            // rule = "/"? (Ref(top_level) / [^/]+ Ref(rule))
            let rule_i = rules.len();
            let rule = Rule::Sequence(vec![
                Rule::Optional(Rule::ByteSequence("/".into()).into()),
                Rule::Choice(vec![
                    Rule::Ref(0),
                    Rule::Sequence(vec![
                        Rule::OneOrMore(
                            Rule::Choice(vec![
                                Rule::ByteRange(u8::MIN, b'/' - 1),
                                Rule::ByteRange(b'/' + 1, u8::MAX),
                            ])
                            .into(),
                        ),
                        Rule::Ref(rule_i),
                    ]),
                ]),
            ]);
            let info = RuleInfo::new("final".into(), rule);
            rules.push(info);
            rules[rule_i].top = true;
        } else {
            rules[0].top = true;
        }

        let rules = Rules::new(rules.into());
        let options = GlobOptions {
            negated,
            directory_only,
        };
        Ok((rules, options))
    }

    pub fn is_match<B: AsRef<[u8]>>(&self, bytes: B) -> bool {
        let mut bytes = bytes.as_ref();
        match self.parser.parse(&mut bytes) {
            Ok(_) => true,
            Err(_e) => false,
        }
    }
}

impl From<GitGlob> for crate::Parser {
    fn from(value: GitGlob) -> Self {
        value.parser.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn glob_rust() {
        let glob = GitGlob::new("**/*.rs").unwrap();
        assert_eq!(glob.is_match(b".hidden"), false);
        assert_eq!(glob.is_match(b"path/to/glob.rs"), true);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn glob_directory() {
        let glob = GitGlob::new("**/node_modules").unwrap();
        assert_eq!(glob.is_match(b"/root/user/node_modules"), true);
        assert_eq!(glob.is_match(b"node_modules"), true);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn glob_zero_dirs() {
        let glob = GitGlob::new("a/**/b").unwrap();
        assert_eq!(glob.is_match(b"a/b"), true);
        assert_eq!(glob.is_match(b"a/x/y/b"), true);
        assert_eq!(glob.is_match(b"b/x/a"), false);
    }

    #[test]
    fn glob_wildcard() {
        let glob = GitGlob::new("*aw*").unwrap();
        assert_eq!(glob.is_match(b"lawyer"), true);
        assert_eq!(glob.is_match(b"the law"), true);
        assert_eq!(glob.is_match(b"the lew"), false);
        assert_eq!(glob.is_match(b"xxxxxxxxxawxxxxxxxx"), true);
        assert_eq!(glob.is_match(b"xxxxxxxxxxxxxxxxx"), false);
    }

    #[test]
    fn glob_hidden() {
        let glob = GitGlob::new(".*").unwrap();
        assert_eq!(glob.is_match(b".hidden"), true);
        assert_eq!(glob.is_match(b"path/to/glob.rs"), false);
        assert_eq!(glob.is_match(b"text/lorem.txt"), false);
    }

    #[test]
    fn glob_question() {
        let glob = GitGlob::new("?at").unwrap();
        assert_eq!(glob.is_match(b"Cat"), true);
        assert_eq!(glob.is_match(b"Bat"), true);
        assert_eq!(glob.is_match(b"ccat"), false);
    }

    #[test]
    fn glob_question_no_separator() {
        let glob = GitGlob::new("foo?ar").unwrap();
        assert_eq!(glob.is_match(b"foobar"), true);
        assert_eq!(glob.is_match(b"foocar"), true);
        assert_eq!(glob.is_match(b"foo/ar"), false);
    }

    #[test]
    fn glob_alt_1() {
        let glob = GitGlob::new("[CB]at").unwrap();
        assert_eq!(glob.is_match(b"Cat"), true);
        assert_eq!(glob.is_match(b"Bat"), true);
        assert_eq!(glob.is_match(b"ccat"), false);
        assert_eq!(glob.is_match(b"Catt"), false);
    }

    #[test]
    fn glob_alt_range() {
        let glob = GitGlob::new("Letter[0-9]").unwrap();
        assert_eq!(glob.is_match(b"Letter8"), true);
        assert_eq!(glob.is_match(b"Letter0"), true);
        assert_eq!(glob.is_match(b"Letter10"), false);
        assert_eq!(glob.is_match(b"Letter"), false);
    }

    #[test]
    fn glob_negate() {
        let glob = GitGlob::new("!Letter[0-9]").unwrap();
        assert_eq!(glob.is_match(b"Letter8"), true);
        assert_eq!(glob.is_match(b"Letter0"), true);
        assert_eq!(glob.is_match(b"Letter10"), false);
        assert_eq!(glob.is_match(b"Letter"), false);
        assert_eq!(glob.options().negated, true)
    }

    #[test]
    fn glob_dir_level() {
        let glob = GitGlob::new("deb/").unwrap();
        assert_eq!(glob.is_match(b"deb"), true);
        assert_eq!(glob.is_match(b"deb/"), true);
        assert_eq!(glob.is_match(b"burbur/hurdurr/deb"), true);
        assert_eq!(glob.is_match(b"/hurdurr/deb"), true);
        assert_eq!(glob.is_match(b"deb/shit"), false);
        assert_eq!(glob.is_match(b"hurdurr/deb/shit"), false);
    }

    #[test]
    fn glob_star_end() {
        let glob = GitGlob::new("perf.data*").unwrap();
        assert_eq!(glob.is_match(b"perf.data"), true);
        assert_eq!(glob.is_match(b"perf.data.old"), true);
        assert_eq!(glob.is_match(b"burbur/hurdurr/perf.data"), true);
        assert_eq!(glob.is_match(b"deb/shit"), false);
        assert_eq!(glob.is_match(b"hurdurr/deb/shit"), false);
    }

    #[test]
    fn glob_recstar_end() {
        let glob = GitGlob::new("perf/**").unwrap();
        assert_eq!(glob.is_match(b"perf/bar"), true);
        assert_eq!(glob.is_match(b"perf/"), true);
        assert_eq!(glob.is_match(b"foo/perf/bar"), false);
        assert_eq!(glob.is_match(b"deb/shit"), false);
        assert_eq!(glob.is_match(b"hurdurr/deb/shit"), false);
    }

    #[test]
    fn glob_direct_match() {
        let glob = GitGlob::new("runtime/config.toml").unwrap();
        assert_eq!(glob.is_match(b"runtime/config.toml"), true);
        assert_eq!(glob.is_match(b"hurdurr/deb/shit"), false);
    }

    #[test]
    fn leading_slash() {
        let glob = GitGlob::new("/config.toml").unwrap();
        assert_eq!(glob.is_match(b"/config.toml"), true);
        assert_eq!(glob.is_match(b"config.toml"), true);
        assert_eq!(glob.is_match(b"/runtime/config.toml"), false);
    }
}
