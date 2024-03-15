use std::{fmt, mem};

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Annotation {
    Whitespaced,
    Show,
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Rule {
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) name: String,
    pub(crate) def: RuleDefinition,
}
impl Rule {
    pub fn apply_whitespaced(&mut self, ws: usize) {
        fn repetition_insert_head(def: &mut RuleDefinition, ws: usize) {
            match def {
                RuleDefinition::OneOrMore(r) => repetition_insert_head(r, ws),
                RuleDefinition::Choice(c) => {
                    let f = &mut c[0];
                    repetition_insert_head(f, ws);
                }
                RuleDefinition::Sequence(s) => {
                    s.insert(0, RuleDefinition::Ref(ws));
                }
                c => {
                    let cur = mem::replace(c, RuleDefinition::Nothing);
                    *c = RuleDefinition::Sequence(vec![RuleDefinition::Ref(ws), cur]);
                }
            }
        }

        fn is_repetition(def: &RuleDefinition) -> bool {
            match def {
                RuleDefinition::OneOrMore(_) => true,
                RuleDefinition::Choice(c) => c.len() == 2 && c[1] == RuleDefinition::Nothing,
                _ => false,
            }
        }

        fn rec(to: &mut RuleDefinition, ws: usize) {
            use RuleDefinition::*;
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
                        if is_repetition(rdef) {
                            repetition_insert_head(rdef, ws);
                        } else {
                            seq.insert(i, RuleDefinition::Ref(ws));
                            i += 1;
                        }

                        i += 1;
                    }
                }
                OneOrMore(m) => rec(m, ws),
                Choice(v) => v.iter_mut().for_each(|r| rec(r, ws)),
                // FollowedBy(_) => todo!(),
                // NotFollowedBy(_) => todo!(),
                _ => {}
            }
        }

        rec(&mut self.def, ws)
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.def)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum RuleDefinition {
    OneOrMore(Box<RuleDefinition>),
    Choice(Vec<RuleDefinition>),
    Sequence(Vec<RuleDefinition>),
    FollowedBy(Box<RuleDefinition>),
    NotFollowedBy(Box<RuleDefinition>),
    CharSequence(String),
    CharRange(char, char),
    Ref(usize),
    Nothing,
}

impl RuleDefinition {
    pub fn is_terminal(&self) -> bool {
        use RuleDefinition::*;
        match self {
            Nothing | CharSequence(_) | CharRange(_, _) => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, RuleDefinition::Nothing)
    }

    pub fn format(&self, rules: &[Rule]) -> String {
        match self {
            RuleDefinition::CharRange(a, b) => format!("[{}..{}]", a, b),
            RuleDefinition::CharSequence(l) => format!("{:?}", l),
            RuleDefinition::Choice(choices) => {
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
            RuleDefinition::Sequence(seq) => {
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
            RuleDefinition::NotFollowedBy(r) => format!("!({})", r.format(rules)),
            RuleDefinition::FollowedBy(r) => format!("&({})", r.format(rules)),
            RuleDefinition::Ref(r) => {
                let rule = &rules[*r].name;
                format!("{}", rule)
            }
            RuleDefinition::OneOrMore(r) => format!("({})+", r.format(rules)),
            RuleDefinition::Nothing => format!("()"),
        }
    }
}

impl fmt::Display for RuleDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleDefinition::CharRange(a, b) => write!(f, "[{}..{}]", a, b),
            RuleDefinition::CharSequence(l) => write!(f, "{:?}", l),
            RuleDefinition::Choice(choices) => {
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
            RuleDefinition::Sequence(seq) => {
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
            RuleDefinition::NotFollowedBy(r) => write!(f, "!({})", r),
            RuleDefinition::FollowedBy(r) => write!(f, "&({})", r),
            RuleDefinition::Ref(r) => write!(f, "r\"{r}\""),
            RuleDefinition::OneOrMore(r) => write!(f, "({})+", r),
            RuleDefinition::Nothing => write!(f, "()"),
        }
    }
}
