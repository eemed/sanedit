use std::{fmt, ops::Deref};

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Annotation {
    Whitespaced,
    Show(Option<String>),
}

#[derive(Debug)]
pub(crate) struct Rules {
    rules: Box<[RuleInfo]>,
}

impl Rules {
    pub fn new(rules: Box<[RuleInfo]>) -> Rules {
        Rules { rules }
    }

    pub fn first_set_of(&self, i: usize) -> Vec<Rule> {
        let ri = &self.rules[i];

        let mut result = vec![];
        let mut seen: Box<[bool]> = vec![false; self.len()].into();
        seen[i] = true;

        first_rec(&ri.rule, self, &mut seen, &mut result);

        result
    }

    pub fn ref_of(&self, i: usize) -> &RuleInfo {
        &self.rules[i]
    }
}

impl fmt::Display for Rules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ri in self.rules.iter() {
            write!(f, "{}: {}", &ri.name, ri.rule.format(&self.rules))?;
        }

        Ok(())
    }
}

/// Pushes terminals to result and returns whether zero chars can match
fn first_rec(rule: &Rule, rules: &Rules, seen: &mut [bool], result: &mut Vec<Rule>) -> bool {
    use Rule::*;
    let mut can_match_zero = false;

    match rule {
        OneOrMore(r) => {
            can_match_zero |= first_rec(r, rules, seen, result);
        }
        ZeroOrMore(r) | Optional(r) => {
            first_rec(r, rules, seen, result);
            can_match_zero = true;
        }
        Choice(choice_rules) => {
            for rule in choice_rules {
                can_match_zero |= first_rec(rule, rules, seen, result);
            }
        }
        Sequence(seq_rules) => {
            for rule in seq_rules {
                can_match_zero = first_rec(rule, rules, seen, result);
                if !can_match_zero {
                    break;
                }
            }
        }
        ByteSequence(s) => {
            result.push(ByteSequence(vec![s[0]]));
        }
        ByteRange(_, _) | ByteAny | UTF8Range(_, _) => result.push(rule.clone()),
        Ref(j) => {
            if !seen[*j] {
                seen[*j] = true;
                let ri = rules.ref_of(*j);
                can_match_zero |= first_rec(&ri.rule, rules, seen, result);
            }
        }
        // FollowedBy(_) => todo!),
        // NotFollowedBy(_) => todo!(),
        _ => {}
    }

    can_match_zero
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
    pub fn display_name(&self) -> &str {
        for ann in &self.annotations {
            match ann {
                Annotation::Show(Some(name)) => return name.as_str(),
                _ => {}
            }
        }

        &self.name
    }

    pub fn show(&self) -> bool {
        self.annotations
            .iter()
            .any(|ann| matches!(ann, Annotation::Show(_)))
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
    Checkpoint,
}

impl Rule {
    pub fn is_terminal(&self) -> bool {
        use Rule::*;
        match self {
            ByteSequence(_) | ByteRange(_, _) | ByteAny | UTF8Range(_, _) => true,
            _ => false,
        }
    }

    pub fn is_repetition(&self) -> bool {
        use Rule::*;
        match self {
            OneOrMore(_) | Optional(_) | ZeroOrMore(_) => true,
            _ => false,
        }
    }

    pub fn format(&self, rules: &[RuleInfo]) -> String {
        use Rule::*;
        match self {
            Choice(choices) => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&choice.format(rules));
                }
                result.push_str(")");

                result
            }
            Sequence(seq) => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&choice.format(rules));
                }
                result.push_str(")");

                result
            }
            NotFollowedBy(r) => format!("!({})", r.format(rules)),
            FollowedBy(r) => format!("&({})", r.format(rules)),
            Ref(r) => {
                let rule = &rules[*r].name;
                format!("{}", rule)
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
                result.push_str("(");
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&format!("{}", choice));
                }
                result.push_str(")");

                write!(f, "{}", result)
            }
            Sequence(seq) => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&format!("{}", choice));
                }
                result.push_str(")");

                write!(f, "{}", result)
            }
            NotFollowedBy(r) => write!(f, "!({})", r),
            FollowedBy(r) => write!(f, "&({})", r),
            Ref(r) => write!(f, "<{r}>"),
            OneOrMore(r) => write!(f, "({})+", r),
            Optional(r) => write!(f, "({})?", r),
            ZeroOrMore(r) => write!(f, "({})*", r),
            ByteSequence(s) => write!(f, "{:02x?}", s),
            ByteRange(a, b) => write!(f, "[{:02x?}..{:02x?}]", a, b),
            ByteAny => write!(f, "."),
            Checkpoint => write!(f, "~"),
        }
    }
}
