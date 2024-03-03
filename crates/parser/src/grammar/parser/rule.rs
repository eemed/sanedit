use std::fmt;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Rule {
    pub(crate) name: String,
    pub(crate) def: RuleDefinition,
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
    Ref(usize),
    Nothing,
}

impl RuleDefinition {
    pub fn is_terminal(&self) -> bool {
        use RuleDefinition::*;
        match self {
            Nothing | CharSequence(_) => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, RuleDefinition::Nothing)
    }

    /// Recursively check if this clause contais refs to another rule
    pub fn has_direct_ref(&self, to: usize) -> bool {
        use RuleDefinition::*;
        match self {
            Sequence(clauses) | Choice(clauses) => {
                for sub in clauses {
                    if sub.has_direct_ref(to) {
                        return true;
                    }
                }

                false
            }
            FollowedBy(r) | NotFollowedBy(r) | OneOrMore(r) => r.has_direct_ref(to),
            Ref(t) => *t == to,
            CharSequence(_) => false,
            Nothing => false,
        }
    }
}

impl fmt::Display for RuleDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleDefinition::CharSequence(l) => write!(f, "\"{}\"", l),
            RuleDefinition::Choice(choices) => {
                let mut result = String::new();
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&format!("{}", choice));
                }

                write!(f, "{}", result)
            }
            RuleDefinition::Sequence(seq) => {
                let mut result = String::new();
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&format!("{}", choice));
                }

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
