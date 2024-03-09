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
}

#[derive(Debug, Clone)]
pub(super) struct SortedClause {
    /// The sorted position of this clause
    pub(super) order: usize,
    pub(super) clause: Rc<Clause>,
}

#[derive(Debug, Clone)]
pub(super) struct Clause {
    pub(super) id: usize,
    pub(super) kind: ClauseKind,
    pub(super) sub: Vec<usize>,
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
    pub fn one_or_more(id: usize, sub: usize) -> Clause {
        Clause {
            id,
            kind: ClauseKind::OneOrMore,
            sub: vec![sub],
        }
    }

    pub fn sequence(id: usize, sub: Vec<usize>) -> Clause {
        Clause {
            id,
            kind: ClauseKind::Sequence,
            sub,
        }
    }

    pub fn choice(id: usize, sub: Vec<usize>) -> Clause {
        Clause {
            id,
            kind: ClauseKind::Choice,
            sub,
        }
    }

    pub fn followed_by(id: usize, sub: usize) -> Clause {
        Clause {
            id,
            kind: ClauseKind::FollowedBy,
            sub: vec![sub],
        }
    }

    pub fn not_followed_by(id: usize, sub: usize) -> Clause {
        Clause {
            id,
            kind: ClauseKind::NotFollowedBy,
            sub: vec![sub],
        }
    }

    pub fn nothing(id: usize) -> Clause {
        Clause {
            id,
            kind: ClauseKind::Nothing,
            sub: vec![],
        }
    }

    pub fn char_sequence(id: usize, string: String) -> Clause {
        Clause {
            id,
            kind: ClauseKind::CharSequence(string),
            sub: vec![],
        }
    }
}

impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ClauseKind::*;
        match &self.kind {
            CharSequence(l) => write!(f, "\"{}\"", l),
            Choice => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in self.sub.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&format!("{}", choice));
                }
                result.push_str(")");

                write!(f, "{}", result)
            }
            Sequence => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in self.sub.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&format!("{}", choice));
                }
                result.push_str(")");

                write!(f, "{}", result)
            }
            NotFollowedBy => write!(f, "!({})", self.sub[0]),
            FollowedBy => write!(f, "&({})", self.sub[0]),
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
        dedup: &mut HashMap<String, usize>,
        clauses: &mut Vec<Clause>,
    ) -> usize {
        let key = format!("{def}");
        match dedup.get(&key) {
            Some(c) => {
                // TODO if encounters a placeholder we should parse this anyway
                // TODO how to determine placeholder?
                c.clone()
            }
            None => {
                let mid = clauses.len();
                clauses.push(Clause::nothing(mid));

                let clause = match def {
                    RuleDefinition::Choice(v) => {
                        let subs = v
                            .iter()
                            .map(|rd| rec_def(rd, rules, dedup, clauses))
                            .collect();
                        Clause::choice(mid, subs)
                    }
                    RuleDefinition::Sequence(v) => {
                        let subs = v
                            .iter()
                            .map(|rd| rec_def(rd, rules, dedup, clauses))
                            .collect();
                        Clause::sequence(mid, subs)
                    }
                    RuleDefinition::OneOrMore(r) => {
                        Clause::one_or_more(mid, rec_def(r, rules, dedup, clauses))
                    }
                    RuleDefinition::FollowedBy(r) => {
                        Clause::followed_by(mid, rec_def(r, rules, dedup, clauses))
                    }
                    RuleDefinition::NotFollowedBy(r) => {
                        Clause::not_followed_by(mid, rec_def(r, rules, dedup, clauses))
                    }
                    RuleDefinition::CharSequence(s) => Clause::char_sequence(mid, s.clone()),
                    RuleDefinition::Ref(r) => {
                        let ref_rule = &rules[*r];
                        let rkey = format!("{}", ref_rule.def);

                        match dedup.get(&rkey) {
                            Some(m) => return *m,
                            None => {
                                // Create placeholder if doesnt exist
                                clauses.push(Clause::nothing(mid));
                                dedup.insert(rkey, mid);
                                return mid;
                            }
                        }
                    }
                    RuleDefinition::Nothing => Clause::nothing(mid),
                };
                println!("Clause: {}: {}", clause.id, clause);

                clauses[mid] = clause;
                dedup.insert(key, mid);

                mid
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

    // for cl in dedup.values() {
    //     println!("CL: {}: {}", cl.id, cl);
    // }

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

    // for t in &top {
    //     println!("TOP: {t}");
    // }

    // for c in &cycles {
    //     println!("CYCLE: {c}");
    // }

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
        topo_clauses_rec(root, by_rule, &mut visited, &mut result);
    }

    // Not the same as len because refs are not in all clauses
    let clauses = result.len();
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
        topo_clauses_rec(&by_rule[i], by_rule, visited, result);
        return;
    }

    let i = me.id as usize;
    if visited[i] {
        return;
    }

    visited[i] = true;

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
