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
}

impl fmt::Display for RuleDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleDefinition::CharRange(a, b) => write!(f, "[{}..{}]", a, b),
            RuleDefinition::CharSequence(l) => write!(f, "\"{}\"", l),
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
