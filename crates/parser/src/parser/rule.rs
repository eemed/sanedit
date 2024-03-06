use std::{
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};

use crate::grammar::{Rule, RuleDefinition};

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
pub(super) struct SortedClause {
    /// The sorted position of this clause
    pub(super) order: usize,
    pub(super) clause: Rc<Clause>,
}

#[derive(Debug, Clone)]
pub(super) struct Clause {
    pub(super) id: u32,
    pub(super) kind: ClauseKind,
    pub(super) sub: Vec<Rc<Clause>>,
}

impl PartialEq for Clause {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Clause {}

impl std::hash::Hash for Clause {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Clause {
    pub fn one_or_more(id: u32, sub: Rc<Clause>) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::OneOrMore,
            sub: vec![sub],
        })
    }

    pub fn sequence(id: u32, sub: Vec<Rc<Clause>>) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::Sequence,
            sub,
        })
    }

    pub fn choice(id: u32, sub: Vec<Rc<Clause>>) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::Choice,
            sub,
        })
    }

    pub fn followed_by(id: u32, sub: Rc<Clause>) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::FollowedBy,
            sub: vec![sub],
        })
    }

    pub fn not_followed_by(id: u32, sub: Rc<Clause>) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::NotFollowedBy,
            sub: vec![sub],
        })
    }

    pub fn nothing(id: u32) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::Nothing,
            sub: vec![],
        })
    }

    pub fn char_sequence(id: u32, string: String) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::CharSequence(string),
            sub: vec![],
        })
    }

    pub fn reference(id: u32, r: usize) -> Rc<Clause> {
        Rc::new(Clause {
            id,
            kind: ClauseKind::Ref(r),
            sub: vec![],
        })
    }
}

impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ClauseKind::*;
        match &self.kind {
            CharSequence(l) => write!(f, "\"{}\"", l),
            Choice => {
                let mut result = String::new();
                for (i, choice) in self.sub.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&format!("{}", choice));
                }

                write!(f, "{}", result)
            }
            Sequence => {
                let mut result = String::new();
                for (i, choice) in self.sub.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&format!("{}", choice));
                }

                write!(f, "{}", result)
            }
            NotFollowedBy => write!(f, "!({})", self.sub[0]),
            FollowedBy => write!(f, "&({})", self.sub[0]),
            Ref(r) => write!(f, "r\"{r}\""),
            OneOrMore => write!(f, "({})+", self.sub[0]),
            Nothing => write!(f, "()"),
        }
    }
}

pub(super) fn preprocess_rules2(rules: Box<[Rule]>) {
    all_clauses(&rules);
}

fn all_clauses(rules: &[Rule]) {
    fn rec_def(
        def: &RuleDefinition,
        rules: &[Rule],
        dedup: &mut HashMap<String, Rc<Clause>>,
        id: &mut u32,
        referenced: &mut HashSet<usize>,
        terminals: &mut Vec<Rc<Clause>>,
    ) -> Rc<Clause> {
        let key = format!("{def}");
        match dedup.get(&key) {
            Some(c) => c.clone(),
            None => {
                let clause = match def {
                    RuleDefinition::Choice(v) => {
                        let subs = v
                            .iter()
                            .map(|rd| rec_def(rd, rules, dedup, id, referenced, terminals))
                            .collect();
                        Clause::choice(*id, subs)
                    }
                    RuleDefinition::Sequence(v) => {
                        let subs = v
                            .iter()
                            .map(|rd| rec_def(rd, rules, dedup, id, referenced, terminals))
                            .collect();
                        Clause::sequence(*id, subs)
                    }
                    RuleDefinition::OneOrMore(r) => Clause::one_or_more(
                        *id,
                        rec_def(r, rules, dedup, id, referenced, terminals),
                    ),
                    RuleDefinition::FollowedBy(r) => Clause::followed_by(
                        *id,
                        rec_def(r, rules, dedup, id, referenced, terminals),
                    ),
                    RuleDefinition::NotFollowedBy(r) => Clause::not_followed_by(
                        *id,
                        rec_def(r, rules, dedup, id, referenced, terminals),
                    ),
                    RuleDefinition::CharSequence(s) => Clause::char_sequence(*id, s.clone()),
                    RuleDefinition::Ref(r) => {
                        referenced.insert(*r);
                        Clause::reference(*id, *r)
                    }
                    RuleDefinition::Nothing => Clause::nothing(*id),
                };

                if def.is_terminal() {
                    terminals.push(clause.clone());
                }

                *id += 1;
                dedup.insert(key, clause.clone());

                clause
            }
        }
    }

    // Deduplicate identical clauses using hashmap
    let mut dedup = HashMap::new();
    // Collect indicies of rules that are referenced in some clause
    let mut referenced: HashSet<usize> = HashSet::new();
    // Collect clauses by rule, each index corresponds to a rule
    let mut by_rule: Box<[Rc<Clause>]> = vec![Clause::nothing(0); rules.len()].into();

    let mut terminals: Vec<Rc<Clause>> = vec![];

    let mut id = 0;
    for (i, rule) in rules.iter().enumerate() {
        let def = &rule.def;
        by_rule[i] = rec_def(
            def,
            rules,
            &mut dedup,
            &mut id,
            &mut referenced,
            &mut terminals,
        );
    }

    // Sort clauses topologically
    sort_topologically(&by_rule, &referenced, terminals, dedup.len())
}

fn sort_topologically(
    by_rule: &[Rc<Clause>],
    referenced: &HashSet<usize>,
    terminals: Vec<Rc<Clause>>,
    total_clauses: usize,
) {
    // Find top level clauses == clauses that are not subclauses of any clause
    let top: HashSet<Rc<Clause>> = {
        // All other clauses are referenced except top level rule ones
        let mut clauses: Vec<(usize, Rc<Clause>)> =
            by_rule.to_vec().iter().cloned().enumerate().collect();

        // Remove all the rules that are refenced in some clause
        clauses.retain(|(i, _)| !referenced.contains(i));
        clauses.into_iter().map(|(_, c)| c).collect()
    };

    // Find possible cycle head clauses
    let cycles = find_cycle_head_clauses(&top, by_rule, total_clauses);

    for t in &top {
        println!("TOP: {t}");
    }

    for c in &cycles {
        println!("CYCLE: {c}");
    }

    let mut roots: HashSet<Rc<Clause>> = HashSet::new();
    roots.extend(top);
    roots.extend(cycles);

    let topo_sorted = topological_clause_order(&roots, terminals, by_rule, total_clauses);
    for s in topo_sorted {
        println!("{}: {}", s.order, s.clause);
    }
}

fn find_cycle_head_clauses(
    top: &HashSet<Rc<Clause>>,
    by_rule: &[Rc<Clause>],
    len: usize,
) -> HashSet<Rc<Clause>> {
    fn detect_clause_cycles_rec(
        clause: &Clause,
        by_rules: &[Rc<Clause>],
        visited: &mut [bool],
        finished: &mut [bool],
        result: &mut HashSet<Rc<Clause>>,
    ) {
        let i = clause.id as usize;
        visited[i] = true;
        let refs = find_clause_refs(&clause);
        for re in refs {
            let sub = &by_rules[re];
            let subi = sub.id as usize;
            if visited[subi] {
                result.insert(sub.clone());
            } else if !finished[subi] {
                detect_clause_cycles_rec(sub.as_ref(), by_rules, visited, finished, result);
            }
        }
        visited[i] = false;
        finished[i] = true;
    }

    let mut result: HashSet<Rc<Clause>> = HashSet::new();
    let mut visited: Box<[bool]> = vec![false; len].into();
    let mut finished: Box<[bool]> = vec![false; len].into();

    for c in top {
        detect_clause_cycles_rec(
            c.as_ref(),
            by_rule,
            &mut visited,
            &mut finished,
            &mut result,
        );
    }

    for c in by_rule {
        detect_clause_cycles_rec(
            c.as_ref(),
            by_rule,
            &mut visited,
            &mut finished,
            &mut result,
        );
    }

    result
}

/// Sort rules to topological order
fn topological_clause_order(
    roots: &HashSet<Rc<Clause>>,
    terminals: Vec<Rc<Clause>>,
    by_rule: &[Rc<Clause>],
    len: usize,
) -> Vec<SortedClause> {
    let mut result = Vec::with_capacity(len);
    let mut visited: Box<[bool]> = vec![false; len].into();
    // Mark terminals as visited
    for term in &terminals {
        let i = term.id as usize;
        visited[i] = true;
    }

    // First put all terminals
    result.extend(terminals);

    // Then the rest
    for root in roots {
        println!("Root: {}", root);
        topo_clauses_rec(root, by_rule, &mut visited, &mut result);
    }

    let clauses = result.len();
    println!("len: {len}, clauses: {clauses}");
    // order = len - i because we are using a max heap instead of min heap
    result
        .into_iter()
        .enumerate()
        .map(|(i, clause)| SortedClause {
            order: clauses - i,
            clause,
        })
        .collect()
}

fn topo_clauses_rec(
    me: &Rc<Clause>,
    by_rule: &[Rc<Clause>],
    visited: &mut [bool],
    result: &mut Vec<Rc<Clause>>,
) {
    if let ClauseKind::Ref(i) = me.kind {
        let sub = &by_rule[i];
        topo_clauses_rec(sub, by_rule, visited, result);
        return;
    }

    let i = me.id as usize;
    if visited[i] {
        println!("{}: Done", me,);
        return;
    }

    visited[i] = true;

    println!("{}: {:?} with {} subclauses", me, me.kind, me.sub.len());
    for sub in &me.sub {
        topo_clauses_rec(sub, by_rule, visited, result);
    }
    result.push(me.clone());
}

fn find_clause_refs(clause: &Clause) -> HashSet<usize> {
    fn rec(clause: &Clause, result: &mut HashSet<usize>) {
        if let ClauseKind::Ref(i) = clause.kind {
            result.insert(i);
        } else {
            clause.sub.iter().for_each(|c| rec(c, result));
        }
    }

    let mut result = HashSet::new();
    rec(clause, &mut result);
    result
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

// /// Detect cycles in the rules and break them.
// /// Sort the rules into topological order
// pub(super) fn preprocess_rules(rules: Box<[Rule]>) -> (Box<[PikaRule]>, Box<[RuleDefinition]>) {
//     // Generally helper functions return arrays where the index is the rule
//     // index and the result is whatever is placed on that index.
//     let top = find_top_rules(&rules);
//     let cycles = detect_cycles(&rules);

//     let mut roots: Vec<usize> = vec![];
//     roots.extend(top);
//     roots.extend(cycles);

//     let order = topological_order(&roots, &rules);
//     let mut parents = find_parents(&rules);

//     let len = rules.len();
//     let prules: Vec<PikaRule> = rules
//         .into_vec()
//         .into_iter()
//         .enumerate()
//         .map(|(i, r)| PikaRule {
//             idx: i,
//             parents: mem::take(&mut parents[i]),
//             // We are using max heap instead of min heap
//             topo_order: len - order[i],
//             rule: r,
//         })
//         .collect();

//     (prules.into(), [].into())
// }

// /// Find rules that refer to us
// fn find_parents(rules: &[Rule]) -> Vec<Vec<usize>> {
//     let mut result = vec![];

//     for i in 0..rules.len() {
//         let mut found = vec![];

//         for (j, r) in rules.iter().enumerate() {
//             if r.def.has_direct_ref(i) {
//                 found.push(j);
//             }
//         }

//         result.push(found);
//     }

//     result
// }

// /// Find rules that are not referenced by other rules.
// fn find_top_rules(rules: &[Rule]) -> HashSet<usize> {
//     let mut result = HashSet::new();

//     'top: for i in 0..rules.len() {
//         for (j, r) in rules.iter().enumerate() {
//             if i == j {
//                 continue;
//             }

//             if r.def.has_direct_ref(i) {
//                 continue 'top;
//             }
//         }

//         result.insert(i);
//     }

//     result
// }

// /// Detect cycles in rules and return their head indices
// fn detect_cycles(rules: &[Rule]) -> HashSet<usize> {
//     let mut result = HashSet::new();
//     let mut visited: Box<[bool]> = vec![false; rules.len()].into();
//     let mut finished: Box<[bool]> = vec![false; rules.len()].into();

//     for i in 0..rules.len() {
//         detect_cycles_rec(i, rules, &mut visited, &mut finished, &mut result);
//     }

//     result
// }

// fn detect_cycles_rec(
//     i: usize,
//     rules: &[Rule],
//     visited: &mut [bool],
//     finished: &mut [bool],
//     result: &mut HashSet<usize>,
// ) {
//     visited[i] = true;
//     let rule = &rules[i];
//     let refs = find_refs(&rule.def);
//     for re in refs {
//         if visited[re] {
//             result.insert(re);
//         } else if !finished[re] {
//             detect_cycles_rec(re, rules, visited, finished, result);
//         }
//     }
//     visited[i] = false;
//     finished[i] = true;
// }

// /// Sort rules to topological order
// fn topological_order(roots: &[usize], rules: &[Rule]) -> Box<[usize]> {
//     let mut visited: Box<[bool]> = vec![false; rules.len()].into();
//     let mut result: Box<[usize]> = vec![0; rules.len()].into();
//     let mut count = 0;

//     for root in roots {
//         topo_rec(*root, rules, &mut visited, &mut result, &mut count);
//     }

//     result.into()
// }

// fn topo_rec(
//     me: usize,
//     rules: &[Rule],
//     visited: &mut [bool],
//     result: &mut [usize],
//     count: &mut usize,
// ) {
//     if visited[me] {
//         return;
//     }

//     visited[me] = true;
//     let rule = &rules[me];
//     let refs = find_refs(&rule.def);
//     for r in refs {
//         topo_rec(r, rules, visited, result, count);
//     }
//     result[me] = *count;
//     *count += 1;
// }

// fn find_refs(clause: &RuleDefinition) -> HashSet<usize> {
//     use RuleDefinition::*;
//     match clause {
//         OneOrMore(r) | FollowedBy(r) | NotFollowedBy(r) => find_refs(r),
//         Choice(v) | Sequence(v) => v.iter().fold(HashSet::new(), |mut acc, c| {
//             acc.extend(&find_refs(c));
//             acc
//         }),
//         Ref(i) => {
//             let mut set = HashSet::new();
//             set.insert(*i);
//             set
//         }
//         _ => HashSet::new(),
//     }
// }
