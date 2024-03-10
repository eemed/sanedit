use std::{
    collections::{HashMap, HashSet},
    fmt, mem,
};

use crate::grammar::{Rule, RuleDefinition};

struct Set {
    items: Box<[bool]>,
}

impl Set {
    pub fn new(n: usize) -> Set {
        Set {
            items: vec![false; n].into(),
        }
    }

    pub fn new_all(n: usize) -> Set {
        Set {
            items: vec![true; n].into(),
        }
    }

    pub fn insert(&mut self, n: usize) {
        self.items[n] = true;
    }

    pub fn remove(&mut self, n: usize) {
        self.items[n] = false;
    }

    pub fn contains(&self, n: usize) -> bool {
        self.items[n]
    }

    pub fn to_vec(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(i, b)| **b)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        SetIter { set: self, i: 0 }
    }

    pub fn union(&mut self, other: Set) {
        for o in other.iter() {
            self.insert(o);
        }
    }
}

struct SetIter<'a> {
    set: &'a Set,
    i: usize,
}

impl<'a> Iterator for SetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.i >= self.set.len() {
                return None;
            }

            let i = self.i;
            let b = self.set.contains(i);
            self.i += 1;

            if b {
                return Some(i);
            }
        }
    }
}

impl fmt::Debug for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        write!(f, "Set {{")?;
        for (i, b) in self.items.iter().enumerate() {
            if *b {
                if !first {
                    write!(f, ", ")?;
                }
                first = false;

                write!(f, "{}", i)?;
            }
        }
        write!(f, "}}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ClauseKind {
    OneOrMore,
    Choice,
    Sequence,
    FollowedBy,
    NotFollowedBy,
    CharSequence(String),
    Nothing,
}

#[derive(Debug, Clone)]
pub(super) struct Clause {
    pub(super) order: usize,
    pub(super) kind: ClauseKind,
    pub(super) sub: Vec<usize>,
    pub(super) parents: Vec<usize>,
    pub(super) can_match_zero: bool,
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
    pub fn one_or_more(sub: usize) -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::OneOrMore,
            sub: vec![sub],
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn sequence(sub: Vec<usize>) -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::Sequence,
            sub,
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn choice(sub: Vec<usize>) -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::Choice,
            sub,
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn followed_by(sub: usize) -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::FollowedBy,
            sub: vec![sub],
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn not_followed_by(sub: usize) -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::NotFollowedBy,
            sub: vec![sub],
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn nothing() -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::Nothing,
            sub: vec![],
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn char_sequence(string: String) -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::CharSequence(string),
            sub: vec![],
            parents: vec![],
            can_match_zero: false,
        }
    }
    pub fn placeholder() -> Clause {
        Clause {
            order: 0,
            kind: ClauseKind::Nothing,
            sub: vec![0],
            parents: vec![],
            can_match_zero: false,
        }
    }

    pub fn is_placeholder(&self) -> bool {
        self.kind == ClauseKind::Nothing && !self.sub.is_empty()
    }

    pub fn is_terminal(&self) -> bool {
        match self.kind {
            ClauseKind::CharSequence(_) => true,
            ClauseKind::Nothing => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self.kind, ClauseKind::Nothing)
    }
}

pub(super) fn preprocess_rules(rules: &[Rule]) -> Box<[Clause]> {
    fn rec(
        def: &RuleDefinition,
        rules: &[Rule],
        dedup: &mut HashMap<String, usize>,
        clauses: &mut Vec<Clause>,
    ) -> usize {
        let key = format!("{def}");
        // Try key, rule ref key, or create new
        let idx = dedup
            .get(&key)
            .copied()
            .or({
                if let RuleDefinition::Ref(r) = def {
                    let rrule = &rules[*r];
                    let rkey = format!("{}", rrule.def);
                    dedup.get(&rkey).copied()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let id = clauses.len();
                clauses.push(Clause::placeholder());
                id
            });

        let clause = &clauses[idx];
        if !clause.is_placeholder() {
            // Already parsed
            return idx;
        }

        let clause = match def {
            RuleDefinition::Choice(v) => {
                let subs = v.iter().map(|rd| rec(rd, rules, dedup, clauses)).collect();
                Clause::choice(subs)
            }
            RuleDefinition::Sequence(v) => {
                let subs = v.iter().map(|rd| rec(rd, rules, dedup, clauses)).collect();
                Clause::sequence(subs)
            }
            RuleDefinition::OneOrMore(r) => Clause::one_or_more(rec(r, rules, dedup, clauses)),
            RuleDefinition::FollowedBy(r) => Clause::followed_by(rec(r, rules, dedup, clauses)),
            RuleDefinition::NotFollowedBy(r) => {
                Clause::not_followed_by(rec(r, rules, dedup, clauses))
            }
            RuleDefinition::CharSequence(s) => Clause::char_sequence(s.clone()),
            // Key is already for the referenced rule
            RuleDefinition::Ref(r) => {
                let rrule = &rules[*r];
                let rkey = format!("{}", rrule.def);
                dedup.insert(rkey, idx);
                Clause::placeholder()
            }
            RuleDefinition::Nothing => Clause::nothing(),
        };

        clauses[idx] = clause;
        dedup.insert(key, idx);

        idx
    }

    let mut dedup = HashMap::new();
    let mut clauses: Vec<Clause> = vec![];
    let mut starts = HashSet::new();

    for rule in rules {
        // println!("====== Rule: {} ========", rule.def);
        let rid = rec(&rule.def, rules, &mut dedup, &mut clauses);
        starts.insert(rid);
    }

    // println!("Starts: {starts:?}");

    sort_topologically(starts, &mut clauses);
    determine_can_match_zero(&mut clauses);
    setup_seed_parents(&mut clauses);

    clauses.into()
}

fn setup_seed_parents(clauses: &mut [Clause]) {
    for i in 0..clauses.len() {
        let clause = &mut clauses[i];
        let subs = mem::take(&mut clause.sub);

        match &clause.kind {
            ClauseKind::Sequence => {
                for s in &subs {
                    let clause = &mut clauses[*s];
                    clause.parents.push(i);

                    if !clause.can_match_zero {
                        break;
                    }
                }
            }
            _ => {
                for s in &subs {
                    let clause = &mut clauses[*s];
                    clause.parents.push(i);
                }
            }
        };

        let clause = &mut clauses[i];
        clause.sub = subs;
    }
}

fn determine_can_match_zero(clauses: &mut [Clause]) {
    let mut cont = true;
    while cont {
        cont = false;

        for i in 0..clauses.len() {
            let clause = &clauses[i];
            let old = clause.can_match_zero;

            let new = match &clause.kind {
                ClauseKind::Choice => clause.sub.iter().any(|i| (&clauses[*i]).can_match_zero),
                ClauseKind::CharSequence(s) => s.is_empty(),
                ClauseKind::Nothing => true,
                _ => clause.sub.iter().all(|i| (&clauses[*i]).can_match_zero),
            };

            cont |= old != new;

            let clause = &mut clauses[i];
            clause.can_match_zero = new;
        }
    }
}

fn sort_topologically(starts: HashSet<usize>, clauses: &mut [Clause]) {
    let top: Set = {
        let mut all = Set::new_all(clauses.len());
        for c in &*clauses {
            for s in &c.sub {
                all.remove(*s);
            }
        }
        all
    };

    let cycles = find_cycle_head_clauses(&top, starts, &clauses);

    // println!("Top: {top:?}");
    // println!("Cycles: {cycles:?}");

    let mut roots = top;
    roots.union(cycles);

    topological_clause_order(&roots, clauses);
}

fn find_cycle_head_clauses(top: &Set, starts: HashSet<usize>, clauses: &[Clause]) -> Set {
    fn detect_clause_cycles_rec(
        i: usize,
        clauses: &[Clause],
        visited: &mut Set,
        finished: &mut Set,
        result: &mut Set,
    ) {
        visited.insert(i);

        let clause = &clauses[i];
        for sub in &clause.sub {
            if visited.contains(*sub) {
                result.insert(sub.clone());
            } else if !finished.contains(*sub) {
                detect_clause_cycles_rec(*sub, clauses, visited, finished, result);
            }
        }

        visited.remove(i);
        finished.insert(i);
    }

    let mut result = Set::new(clauses.len());
    let mut visited = Set::new(clauses.len());
    let mut finished = Set::new(clauses.len());

    for c in top.iter() {
        detect_clause_cycles_rec(c, clauses, &mut visited, &mut finished, &mut result);
    }

    for c in starts {
        detect_clause_cycles_rec(c, clauses, &mut visited, &mut finished, &mut result);
    }

    result
}

/// Sort rules to topological order
fn topological_clause_order(roots: &Set, clauses: &mut [Clause]) {
    let len = clauses.len();
    let mut order = 0;

    let terminals = {
        let mut terms = Set::new(len);
        for (i, clause) in clauses.iter().enumerate() {
            if clause.is_terminal() {
                terms.insert(i);
            }
        }
        terms
    };

    // First put all terminals
    for term in terminals.iter() {
        clauses[term].order = len - 1 - order;
        order += 1;
    }

    // Mark terminals as visited
    let mut visited = terminals;

    // Then the rest
    for root in roots.iter() {
        topo_clauses_rec(root, clauses, &mut visited, &mut order);
    }
}

fn topo_clauses_rec(i: usize, clauses: &mut [Clause], visited: &mut Set, order: &mut usize) {
    let len = clauses.len();
    if visited.contains(i) {
        return;
    }

    visited.insert(i);

    let clause = &mut clauses[i];
    let subs = mem::take(&mut clause.sub);
    for sub in &subs {
        topo_clauses_rec(*sub, clauses, visited, order);
    }

    let clause = &mut clauses[i];
    clause.sub = subs;
    clause.order = len - 1 - *order;
    *order += 1;
}

// ---------------------------------------------
#[derive(Debug, Clone, Hash, Eq, Ord)]
pub(super) struct PikaRule {
    /// Index which refers to this rule
    pub(super) idx: usize,
    /// Topological ordering of this rule
    pub(super) topo_order: usize,
    /// Indices to parent rules that reference this rule
    pub(super) parents: Vec<usize>,
    /// Underlying rule
    pub(super) rule: Rule,
}

impl PartialEq for PikaRule {
    fn eq(&self, other: &Self) -> bool {
        self.topo_order.eq(&other.topo_order)
    }
}

impl PartialOrd for PikaRule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.topo_order.partial_cmp(&other.topo_order)
    }
}
