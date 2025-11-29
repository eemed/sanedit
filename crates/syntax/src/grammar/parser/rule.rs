use std::{fmt, ops::Deref};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{grammar::lexer::Lexer, Operation};

use super::GrammarParser;

/// Annotation a rule has in the peg grammar
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Annotation {
    Whitespaced,
    Show,
    /// Parse matches of this rule using another parser
    Inject(Option<String>),
    /// This matches injection language that can be used with Inject
    InjectionLanguage,
    Other(String, Option<String>),
}

/// Ordered set of rules, rules are identified by their indices
#[derive(Debug, Clone)]
pub(crate) struct Rules {
    rules: Box<[RuleInfo]>,
}

impl Rules {
    pub fn new(rules: Box<[RuleInfo]>) -> Rules {
        Rules { rules }
    }

    pub fn parse<R: std::io::Read>(read: R) -> anyhow::Result<Rules> {
        let mut lex = Lexer::new(read);
        let token = lex.next()?;
        let parser = GrammarParser {
            lex,
            token,
            rules: vec![],
            indices: FxHashMap::default(),
            seen: FxHashSet::default(),
        };
        parser.parse()
    }

    pub fn injection_ids(&self) -> Option<Box<[bool]>> {
        let mut found = false;
        let mut result = vec![false; self.rules.len()].into_boxed_slice();
        for (i, rule) in self.rules.iter().enumerate() {
            if rule
                .annotations
                .iter()
                .any(|ann| matches!(ann, Annotation::Inject(..) | Annotation::InjectionLanguage))
            {
                found = true;
                result[i] = true;
            }
        }

        if !found {
            return None;
        }

        Some(result)
    }
}

impl fmt::Display for Rules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ri in self.rules.iter() {
            writeln!(f, "{}: {}", &ri.name, ri.rule.format(&self.rules))?;
        }

        Ok(())
    }
}

impl Deref for Rules {
    type Target = [RuleInfo];

    fn deref(&self) -> &Self::Target {
        &self.rules
    }
}

/// A Rule with extra information about it
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct RuleInfo {
    pub(crate) top: bool,
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) name: String,
    pub(crate) rule: Rule,
}

impl RuleInfo {
    /// Common way to create a rule
    pub fn new(name: &str, rule: Rule) -> RuleInfo {
        RuleInfo {
            top: false,
            annotations: vec![],
            name: name.into(),
            rule,
        }
    }

    pub fn show(&self) -> bool {
        self.annotations.iter().any(|ann| {
            matches!(
                ann,
                Annotation::Show | Annotation::Inject(..) | Annotation::InjectionLanguage
            )
        })
    }

    pub fn apply_whitespaced(&mut self, ws: usize) {
        fn repetition_insert_head(rule: &mut Rule, ws: usize) {
            use Rule::*;
            match rule {
                Optional(r) | ZeroOrMore(r) | OneOrMore(r) => repetition_insert_head(r, ws),
                Choice(rules) => {
                    let first = &mut rules[0];
                    repetition_insert_head(first, ws);
                }
                Sequence(rules) => {
                    rules.insert(0, Rule::Ref(ws));
                }
                crule => {
                    let cur = crule.clone();
                    *crule = Rule::Sequence(vec![Rule::Ref(ws), cur]);
                }
            }
        }

        fn rec(to: &mut Rule, ws: usize) {
            use Rule::*;
            match to {
                Sequence(seq) => {
                    for r in seq.iter_mut() {
                        rec(r, ws);
                    }

                    if seq.len() == 1 {
                        return;
                    }

                    let mut i = 1;
                    while i < seq.len() {
                        let rdef = &mut seq[i];
                        if rdef.is_repetition() {
                            repetition_insert_head(rdef, ws);
                        } else {
                            seq.insert(i, Ref(ws));
                            i += 1;
                        }

                        i += 1;
                    }
                }
                Choice(rules) => rules.iter_mut().for_each(|r| rec(r, ws)),
                OneOrMore(rule) | Optional(rule) | ZeroOrMore(rule) | NotFollowedBy(rule)
                | FollowedBy(rule) => rec(rule, ws),
                _ => {}
            }
        }

        rec(&mut self.rule, ws)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Rule {
    Optional(Box<Rule>),
    ZeroOrMore(Box<Rule>),
    OneOrMore(Box<Rule>),
    Choice(Vec<Rule>),
    Sequence(Vec<Rule>),
    FollowedBy(Box<Rule>),
    NotFollowedBy(Box<Rule>),
    ByteSequence(Vec<u8>),
    /// Inclusive byte range.
    /// This is separate from UTF8Range to ease parser optimization
    /// Technically it could be removed as UTF8Range covers all byte ranges this
    /// can represent
    ByteRange(u8, u8),
    ByteAny,
    /// Inclusive UTF8 range
    UTF8Range(char, char),
    Ref(usize),
    /// Add an instruction directly
    Embed(Operation),
}

impl Rule {
    pub fn is_byte_range_or_single_byte(&self) -> bool {
        match self {
            Rule::ByteSequence(vec) => vec.len() == 1,
            Rule::ByteRange(_, _) => true,
            _ => false,
        }
    }

    pub fn is_repetition(&self) -> bool {
        use Rule::*;
        matches!(self, OneOrMore(_) | Optional(_) | ZeroOrMore(_))
    }

    pub fn format(&self, rules: &[RuleInfo]) -> String {
        use Rule::*;
        match self {
            Choice(choices) => {
                let mut result = String::new();
                result.push('(');
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&choice.format(rules));
                }
                result.push(')');

                result
            }
            Sequence(seq) => {
                let mut result = String::new();
                result.push('(');
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push(' ');
                    }

                    result.push_str(&choice.format(rules));
                }
                result.push(')');

                result
            }
            NotFollowedBy(r) => format!("!({})", r.format(rules)),
            FollowedBy(r) => format!("&({})", r.format(rules)),
            Ref(r) => {
                let rule = &rules[*r].name;
                rule.to_string()
            }
            OneOrMore(r) => format!("({})+", r.format(rules)),
            Optional(r) => format!("({})?", r.format(rules)),
            ZeroOrMore(r) => format!("({})*", r.format(rules)),
            _ => format!("{}", self),
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Rule::*;
        match self {
            UTF8Range(a, b) => {
                write!(f, "[{}..{}]", a.escape_unicode(), b.escape_unicode())
            }
            Choice(choices) => {
                let mut result = String::new();
                result.push('(');
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&format!("{}", choice));
                }
                result.push(')');

                write!(f, "{}", result)
            }
            Sequence(seq) => {
                let mut result = String::new();
                result.push('(');
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push(' ');
                    }

                    result.push_str(&format!("{}", choice));
                }
                result.push(')');

                write!(f, "{}", result)
            }
            NotFollowedBy(r) => write!(f, "!({})", r),
            FollowedBy(r) => write!(f, "&({})", r),
            Ref(r) => write!(f, "<{r}>"),
            OneOrMore(r) => write!(f, "({})+", r),
            Optional(r) => write!(f, "({})?", r),
            ZeroOrMore(r) => write!(f, "({})*", r),
            ByteSequence(s) => write!(f, "{:?}", s),
            ByteRange(a, b) => write!(f, "[{:?}..{:?}]", a, b),
            ByteAny => write!(f, "."),
            Embed(operation) => write!(f, "<{operation:?}>"),
        }
    }
}
