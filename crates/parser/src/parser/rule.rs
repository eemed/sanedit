use std::{
    collections::{HashMap, HashSet},
    mem,
    rc::Rc,
};

use crate::grammar::{Rule, RuleDefinition};

// #[derive(Debug, Clone)]
// pub(super) enum Clause {
//     OneOrMore(Rc<Clause>),
//     Choice(Vec<Rc<Clause>>),
//     Sequence(Vec<Rc<Clause>>),
//     FollowedBy(Rc<Clause>),
//     NotFollowedBy(Rc<Clause>),
//     CharSequence(String),
//     Nothing,
// }

#[derive(Debug, Clone)]
pub(super) enum ClauseKind {
    OneOrMore,
    Choice,
    Sequence,
    FollowedBy,
    NotFollowedBy,
    CharSequence(String),
    Nothing,
    Ref(usize),
}

#[derive(Debug, Clone)]
pub(super) struct Clause {
    /// Topological ordering of this rule
    pub(super) topo_order: usize,
    pub(super) kind: ClauseKind,
    pub(super) sub: Vec<Rc<Clause>>,
}

impl Clause {
    pub fn one_or_more(sub: Rc<Clause>) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::OneOrMore,
            sub: vec![sub],
        })
    }

    pub fn sequence(sub: Vec<Rc<Clause>>) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::Sequence,
            sub,
        })
    }

    pub fn choice(sub: Vec<Rc<Clause>>) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::Choice,
            sub,
        })
    }

    pub fn followed_by(sub: Rc<Clause>) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::FollowedBy,
            sub: vec![sub],
        })
    }

    pub fn not_followed_by(sub: Rc<Clause>) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::NotFollowedBy,
            sub: vec![sub],
        })
    }

    pub fn nothing() -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::Nothing,
            sub: vec![],
        })
    }

    pub fn char_sequence(string: String) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::CharSequence(string),
            sub: vec![],
        })
    }

    pub fn reference(r: usize) -> Rc<Clause> {
        Rc::new(Clause {
            topo_order: 0,
            kind: ClauseKind::Ref(r),
            sub: vec![],
        })
    }
}

pub(super) fn preprocess_rules2(rules: Box<[Rule]>) {
    let top = find_top_rules(&rules);
    let cycles = detect_cycles(&rules);

    all_clauses(&rules);

    let mut roots: Vec<usize> = vec![];
    roots.extend(top);
    roots.extend(cycles);

    // let order = topological_order(&roots, &rules);

    // for o in order.iter() {
    //     let rule = &rules[*o];
    //     println!("{rule}");
    // }
}

fn all_clauses(rules: &[Rule]) {
    fn rec_def(
        def: &RuleDefinition,
        rules: &[Rule],
        dedup: &mut HashMap<String, Rc<Clause>>,
    ) -> Rc<Clause> {
        let key = format!("{def}");
        match dedup.get(&key) {
            Some(c) => c.clone(),
            None => {
                let clause = match def {
                    RuleDefinition::Choice(v) => {
                        let subs = v.iter().map(|rd| rec_def(rd, rules, dedup)).collect();
                        Clause::choice(subs)
                    }
                    RuleDefinition::Sequence(v) => {
                        let subs = v.iter().map(|rd| rec_def(rd, rules, dedup)).collect();
                        Clause::sequence(subs)
                    }
                    RuleDefinition::OneOrMore(r) => Clause::one_or_more(rec_def(r, rules, dedup)),
                    RuleDefinition::FollowedBy(r) => Clause::followed_by(rec_def(r, rules, dedup)),
                    RuleDefinition::NotFollowedBy(r) => {
                        Clause::not_followed_by(rec_def(r, rules, dedup))
                    }
                    RuleDefinition::CharSequence(s) => Clause::char_sequence(s.clone()),
                    RuleDefinition::Ref(r) => Clause::reference(*r),
                    RuleDefinition::Nothing => Clause::nothing(),
                };
                dedup.insert(key, clause.clone());

                clause
            }
        }
    }

    let mut dedup = HashMap::new();
    let mut clauses: Box<[Rc<Clause>]> = vec![Clause::nothing(); rules.len()].into();
    for (i, rule) in rules.iter().enumerate() {
        let def = &rule.def;
        clauses[i] = rec_def(def, rules, &mut dedup);
    }

    // Replace refs.. how?

    println!("Clauses: {clauses:?}");
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

/// Detect cycles in the rules and break them.
/// Sort the rules into topological order
pub(super) fn preprocess_rules(rules: Box<[Rule]>) -> (Box<[PikaRule]>, Box<[RuleDefinition]>) {
    // Generally helper functions return arrays where the index is the rule
    // index and the result is whatever is placed on that index.
    let top = find_top_rules(&rules);
    let cycles = detect_cycles(&rules);

    let mut roots: Vec<usize> = vec![];
    roots.extend(top);
    roots.extend(cycles);

    let order = topological_order(&roots, &rules);
    let mut parents = find_parents(&rules);

    let len = rules.len();
    let prules: Vec<PikaRule> = rules
        .into_vec()
        .into_iter()
        .enumerate()
        .map(|(i, r)| PikaRule {
            idx: i,
            parents: mem::take(&mut parents[i]),
            // We are using max heap instead of min heap
            topo_order: len - order[i],
            rule: r,
        })
        .collect();

    (prules.into(), [].into())
}

/// Find rules that refer to us
fn find_parents(rules: &[Rule]) -> Vec<Vec<usize>> {
    let mut result = vec![];

    for i in 0..rules.len() {
        let mut found = vec![];

        for (j, r) in rules.iter().enumerate() {
            if r.def.has_direct_ref(i) {
                found.push(j);
            }
        }

        result.push(found);
    }

    result
}

/// Find rules that are not referenced by other rules.
fn find_top_rules(rules: &[Rule]) -> HashSet<usize> {
    let mut result = HashSet::new();

    'top: for i in 0..rules.len() {
        for (j, r) in rules.iter().enumerate() {
            if i == j {
                continue;
            }

            if r.def.has_direct_ref(i) {
                continue 'top;
            }
        }

        result.insert(i);
    }

    result
}

/// Detect cycles in rules and return their head indices
fn detect_cycles(rules: &[Rule]) -> HashSet<usize> {
    let mut result = HashSet::new();
    let mut visited: Box<[bool]> = vec![false; rules.len()].into();
    let mut finished: Box<[bool]> = vec![false; rules.len()].into();

    for i in 0..rules.len() {
        detect_cycles_rec(i, rules, &mut visited, &mut finished, &mut result);
    }

    result
}

fn detect_cycles_rec(
    i: usize,
    rules: &[Rule],
    visited: &mut [bool],
    finished: &mut [bool],
    result: &mut HashSet<usize>,
) {
    visited[i] = true;
    let rule = &rules[i];
    let refs = find_refs(&rule.def);
    for re in refs {
        if visited[re] {
            result.insert(re);
        } else if !finished[re] {
            detect_cycles_rec(re, rules, visited, finished, result);
        }
    }
    visited[i] = false;
    finished[i] = true;
}

/// Sort rules to topological order
fn topological_order(roots: &[usize], rules: &[Rule]) -> Box<[usize]> {
    let mut visited: Box<[bool]> = vec![false; rules.len()].into();
    let mut result: Box<[usize]> = vec![0; rules.len()].into();
    let mut count = 0;

    for root in roots {
        topo_rec(*root, rules, &mut visited, &mut result, &mut count);
    }

    result.into()
}

fn topo_rec(
    me: usize,
    rules: &[Rule],
    visited: &mut [bool],
    result: &mut [usize],
    count: &mut usize,
) {
    if visited[me] {
        return;
    }

    visited[me] = true;
    let rule = &rules[me];
    let refs = find_refs(&rule.def);
    for r in refs {
        topo_rec(r, rules, visited, result, count);
    }
    result[me] = *count;
    *count += 1;
}

fn find_refs(clause: &RuleDefinition) -> HashSet<usize> {
    use RuleDefinition::*;
    match clause {
        OneOrMore(r) | FollowedBy(r) | NotFollowedBy(r) => find_refs(r),
        Choice(v) | Sequence(v) => v.iter().fold(HashSet::new(), |mut acc, c| {
            acc.extend(&find_refs(c));
            acc
        }),
        Ref(i) => {
            let mut set = HashSet::new();
            set.insert(*i);
            set
        }
        _ => HashSet::new(),
    }
}
