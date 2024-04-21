use std::fmt;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Annotation {
    Whitespaced,
    Show,
    Alias(String),
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Rule {
    pub(crate) annotations: Vec<Annotation>,
    pub(crate) name: String,
    pub(crate) def: RuleDefinition,
}
impl Rule {
    pub fn name(&self) -> String {
        for ann in &self.annotations {
            match ann {
                Annotation::Alias(name) => return name.clone(),
                _ => {}
            }
        }

        self.name.clone()
    }

    pub fn apply_whitespaced(&mut self, ws: usize) {
        fn repetition_insert_head(def: &mut RuleDefinition, ws: usize) {
            use RuleDefinition::*;
            match def {
                Optional(r) | ZeroOrMore(r) | OneOrMore(r) => repetition_insert_head(r, ws),
                Choice(c) => {
                    let f = &mut c[0];
                    repetition_insert_head(f, ws);
                }
                Sequence(s) => {
                    s.insert(0, RuleDefinition::Ref(ws));
                }
                c => {
                    let cur = c.clone();
                    *c = RuleDefinition::Sequence(vec![RuleDefinition::Ref(ws), cur]);
                }
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
                        if rdef.is_repetition() {
                            repetition_insert_head(rdef, ws);
                        } else {
                            seq.insert(i, Ref(ws));
                            i += 1;
                        }

                        i += 1;
                    }
                }
                OneOrMore(m) => rec(m, ws),
                Optional(m) => rec(m, ws),
                ZeroOrMore(m) => rec(m, ws),
                Choice(v) => v.iter_mut().for_each(|r| rec(r, ws)),
                FollowedBy(m) => rec(m, ws),
                NotFollowedBy(m) => rec(m, ws),
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
    Optional(Box<RuleDefinition>),
    ZeroOrMore(Box<RuleDefinition>),
    OneOrMore(Box<RuleDefinition>),
    Choice(Vec<RuleDefinition>),
    Sequence(Vec<RuleDefinition>),
    FollowedBy(Box<RuleDefinition>),
    NotFollowedBy(Box<RuleDefinition>),
    CharSequence(String),
    CharRange(char, char),
    Ref(usize),
}

impl RuleDefinition {
    pub fn is_terminal(&self) -> bool {
        use RuleDefinition::*;
        match self {
            CharSequence(_) | CharRange(_, _) => true,
            _ => false,
        }
    }

    pub fn is_repetition(&self) -> bool {
        use RuleDefinition::*;
        match self {
            OneOrMore(_) | Optional(_) | ZeroOrMore(_) => true,
            _ => false,
        }
    }

    pub fn format(&self, rules: &[Rule]) -> String {
        use RuleDefinition::*;
        match self {
            CharRange(a, b) => format!("[{}..{}]", a, b),
            CharSequence(l) => format!("{:?}", l),
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
        }
    }
}

impl fmt::Display for RuleDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RuleDefinition::*;
        match self {
            CharRange(a, b) => write!(f, "[{}..{}]", a, b),
            CharSequence(l) => write!(f, "{:?}", l),
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
            Ref(r) => write!(f, "r\"{r}\""),
            OneOrMore(r) => write!(f, "({})+", r),
            Optional(r) => write!(f, "({})?", r),
            ZeroOrMore(r) => write!(f, "({})*", r),
        }
    }
}
