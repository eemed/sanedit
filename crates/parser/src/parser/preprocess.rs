use std::{collections::HashMap, mem};

use anyhow::bail;

use crate::grammar::{Annotation, Rule, RuleDefinition};

use super::{
    clause::{Clause, ClauseKind},
    set::Set,
};

#[derive(Debug)]
pub(crate) struct Clauses {
    pub(crate) names: HashMap<usize, Vec<String>>,
    pub(crate) clauses: Box<[Clause]>,
}

pub(super) fn preprocess_rules(rules: &[Rule]) -> anyhow::Result<Clauses> {
    fn rec(
        def: &RuleDefinition,
        rules: &[Rule],
        dedup: &mut HashMap<String, usize>,
        clauses: &mut Vec<Clause>,
    ) -> usize {
        let mut cdef = def;
        // Dereference any refs
        while let RuleDefinition::Ref(r) = cdef {
            let rrule = &rules[*r];
            cdef = &rrule.def;
        }

        let key = format!("{cdef}");
        let idx = dedup.get(&key).copied().unwrap_or_else(|| {
            let id = clauses.len();
            clauses.push(Clause::placeholder());
            dedup.insert(key, id);
            id
        });

        let clause = &clauses[idx];
        if !clause.is_placeholder() || matches!(def, RuleDefinition::Ref(_)) {
            // Already parsed or a reference that will be parsed in the future
            return idx;
        }

        let mut clause = match def {
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
            RuleDefinition::Nothing => Clause::nothing(),
            RuleDefinition::CharRange(a, b) => Clause::char_range(*a, *b),
            _ => unreachable!("Encountered unexpected rule definition: {def}"),
        };

        clause.idx = idx;
        clauses[idx] = clause;

        idx
    }

    let mut dedup = HashMap::new();
    let mut clauses: Vec<Clause> = vec![];
    let mut names = HashMap::new();

    for rule in rules {
        let rid = rec(&rule.def, rules, &mut dedup, &mut clauses);

        if rule.annotations.contains(&Annotation::Show) {
            clauses[rid].show = true;
        }

        let val: &mut Vec<String> = names.entry(rid).or_default();
        val.push(rule.name.clone());
    }

    for rule in rules.iter() {
        println!("{} = {}", rule.name, rule.def.format(&rules));
    }

    let rule_starts = {
        let mut set = Set::new(clauses.len());
        for v in names.keys() {
            set.insert(*v);
        }
        set
    };

    sort_topologically(&rule_starts, &mut clauses);
    determine_can_match_zero(&mut clauses);
    setup_seed_parents(&mut clauses);

    for cl in &clauses {
        println!("Clause: {cl:?}");
    }

    validate(&clauses)?;

    Ok(Clauses {
        names,
        clauses: clauses.into(),
    })
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
                ClauseKind::CharRange(a, b) => a > b,
                ClauseKind::Nothing => true,
                _ => clause.sub.iter().all(|i| (&clauses[*i]).can_match_zero),
            };

            cont |= old != new;

            let clause = &mut clauses[i];
            clause.can_match_zero = new;
        }
    }
}

fn sort_topologically(rule_starts: &Set, clauses: &mut [Clause]) {
    let top: Set = {
        let mut all = Set::new_all(clauses.len());
        for c in &*clauses {
            for s in &c.sub {
                all.remove(*s);
            }
        }
        all
    };

    let cycles = find_cycle_head_clauses(&top, rule_starts, &clauses);

    // println!("Top: {top:?}");
    // println!("Cycles: {cycles:?}");

    let mut roots = top;
    roots.union(cycles);

    topological_clause_order(&roots, clauses);
}

fn find_cycle_head_clauses(top: &Set, rule_starts: &Set, clauses: &[Clause]) -> Set {
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

    for c in rule_starts.iter() {
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

fn validate(clauses: &[Clause]) -> anyhow::Result<()> {
    let mut errors = vec![];
    for clause in clauses {
        if clause.is_placeholder() {
            errors.push(format!("Placeholder clause not replaced: {:?}", clause));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        bail!(errors.join("\n"))
    }
}
