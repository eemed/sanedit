#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClauseKind {
    OneOrMore,
    Choice,
    Sequence,
    FollowedBy,
    NotFollowedBy,
    CharSequence(String),
    CharRange(char, char),
    Nothing,
}

#[derive(Debug, Clone)]
pub(crate) struct Clause {
    pub(crate) show: bool,
    pub(crate) idx: usize,
    pub(crate) order: usize,
    pub(crate) kind: ClauseKind,
    pub(crate) sub: Vec<usize>,
    pub(crate) parents: Vec<usize>,
    pub(crate) can_match_zero: bool,
}

impl PartialEq for Clause {
    fn eq(&self, other: &Self) -> bool {
        self.order.eq(&other.order)
    }
}

impl Eq for Clause {}

impl PartialOrd for Clause {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.order.partial_cmp(&other.order)
    }
}

impl Ord for Clause {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

impl Clause {
    pub fn nothing() -> Clause {
        Clause {
            show: false,
            idx: 0,
            order: 0,
            kind: ClauseKind::Nothing,
            sub: vec![],
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn one_or_more(sub: usize) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::OneOrMore;
        clause.sub.push(sub);
        clause
    }

    pub fn sequence(sub: Vec<usize>) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::Sequence;
        clause.sub = sub;
        clause
    }

    pub fn choice(sub: Vec<usize>) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::Choice;
        clause.sub = sub;
        clause
    }

    pub fn followed_by(sub: usize) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::FollowedBy;
        clause.sub.push(sub);
        clause
    }

    pub fn not_followed_by(sub: usize) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::NotFollowedBy;
        clause.sub.push(sub);
        clause
    }

    pub fn char_sequence(string: String) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::CharSequence(string);
        clause
    }

    pub fn char_range(from: char, to: char) -> Clause {
        let mut clause = Clause::nothing();
        clause.kind = ClauseKind::CharRange(from, to);
        clause
    }

    pub fn placeholder() -> Clause {
        let mut clause = Clause::nothing();
        clause.sub.push(0);
        clause
    }

    pub fn is_placeholder(&self) -> bool {
        self.kind == ClauseKind::Nothing && !self.sub.is_empty()
    }

    pub fn is_terminal(&self) -> bool {
        use ClauseKind::*;
        match self.kind {
            CharSequence(_) => true,
            CharRange(_, _) => true,
            Nothing => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self.kind, ClauseKind::Nothing)
    }

    pub fn should_lex(&self, clauses: &[Clause]) -> bool {
        use ClauseKind::*;
        match self.kind {
            OneOrMore => {
                let sub = &clauses[self.sub[0]];
                sub.is_terminal()
            }
            Choice => {
                if self.sub.len() != 2 {
                    return false;
                }
                // Is repetition
                let first = &clauses[self.sub[0]];
                let second = &clauses[self.sub[1]];
                first.is_terminal() && second.is_nothing()
            }
            CharSequence(_) => true,
            CharRange(_, _) => true,
            _ => false,
        }
    }
}
