use std::mem;

use anyhow::bail;
use rustc_hash::FxHashMap;

use crate::{
    grammar::{Annotation, Rule, RuleDefinition},
    pika_parser::clause::SubClause,
};

use super::{
    clause::{Clause, ClauseKind},
    set::Set,
};

#[derive(Debug)]
pub(crate) struct Clauses {
    pub(crate) names: FxHashMap<usize, String>,
    pub(crate) clauses: Box<[Clause]>,
}

pub(super) fn preprocess_rules(rules: &[Rule]) -> anyhow::Result<Clauses> {
    fn sub_rec(
        def: &RuleDefinition,
        rules: &[Rule],
        dedup: &mut FxHashMap<String, usize>,
        clauses: &mut Vec<Clause>,
    ) -> SubClause {
        rec(def, None, rules, dedup, clauses)
    }

    fn rec(
        def: &RuleDefinition,
        mut alias: Option<String>,
        rules: &[Rule],
        dedup: &mut FxHashMap<String, usize>,
        clauses: &mut Vec<Clause>,
    ) -> SubClause {
        use RuleDefinition::*;

        let mut cdef = def;
        // Dereference any refs
        while let Ref(r) = cdef {
            let rrule = &rules[*r];
            cdef = &rrule.def;

            for ann in &rrule.annotations {
                match ann {
                    Annotation::Show(Some(n)) => alias = Some(n.clone()),
                    _ => {}
                }
            }
        }

        let key = format!("{cdef}");
        let idx = dedup.get(&key).copied().unwrap_or_else(|| {
            let id = clauses.len();
            clauses.push(Clause::placeholder());
            dedup.insert(key, id);
            id
        });

        let clause = &clauses[idx];
        if !clause.is_placeholder() || matches!(def, Ref(_)) {
            // Already parsed or a reference that will be parsed in the future
            let mut result = SubClause::new(idx);
            result.alias = alias;
            return result;
        }

        let mut clause = match def {
            Choice(v) => {
                let subs = v
                    .iter()
                    .map(|rd| sub_rec(rd, rules, dedup, clauses))
                    .collect();
                Clause::choice(subs)
            }
            Sequence(v) => {
                let subs = v
                    .iter()
                    .map(|rd| sub_rec(rd, rules, dedup, clauses))
                    .collect();
                Clause::sequence(subs)
            }
            OneOrMore(r) => Clause::one_or_more(sub_rec(r, rules, dedup, clauses)),
            FollowedBy(r) => Clause::followed_by(sub_rec(r, rules, dedup, clauses)),
            NotFollowedBy(r) => Clause::not_followed_by(sub_rec(r, rules, dedup, clauses)),
            CharSequence(s) => Clause::char_sequence(s.clone()),
            CharRange(a, b) => Clause::char_range(*a, *b),
            Optional(r) => {
                let sub = sub_rec(r, rules, dedup, clauses);
                // One or Nothing
                Clause::choice(vec![sub, SubClause::new(0)])
            }
            ZeroOrMore(r) => {
                let rule = r.clone();
                let plus = sub_rec(&RuleDefinition::OneOrMore(rule), rules, dedup, clauses);
                // OneOrMore or Nothing
                Clause::choice(vec![plus, SubClause::new(0)])
            }
            _ => unreachable!("Encountered unexpected rule definition: {def}"),
        };

        clause.idx = idx;
        clauses[idx] = clause;

        let mut result = SubClause::new(idx);
        result.alias = alias;
        result
    }

    let mut dedup = FxHashMap::default();
    let mut clauses: Vec<Clause> = vec![];
    let mut names = FxHashMap::default();

    // Push nothing as index 0
    clauses.push(Clause::nothing());

    for rule in rules {
        let mut name = rule.name.clone();
        let mut show = false;

        for ann in &rule.annotations {
            match ann {
                Annotation::Show(nname) => {
                    if let Some(n) = nname {
                        name = n.clone();
                    }
                    show = true;
                }
                _ => {}
            }
        }

        let cl = rec(&rule.def, Some(name), rules, &mut dedup, &mut clauses);

        clauses[cl.idx].show = show;
        clauses[cl.idx].top = rule.top;

        if rule.top {
            log::info!("TOP: {rule:?}");
        }

        let val: &mut String = names.entry(cl.idx).or_default();
        *val = rule.name.clone();
    }

    let rule_starts = {
        let mut set = Set::new(clauses.len());
        for v in names.keys() {
            set.insert(*v);
        }
        set
    };

    solve_not_followed_by(&mut clauses);
    sort_topologically(&rule_starts, &mut clauses);
    determine_can_match_zero(&mut clauses);
    setup_seed_parents(&mut clauses);
    validate(&clauses)?;

    // debug_print(&rules, &clauses);
    // debug_log(&rules, &clauses);

    Ok(Clauses {
        names,
        clauses: clauses.into(),
    })
}

fn debug_log(rules: &[Rule], clauses: &[Clause]) {
    log::debug!("----- Rules ------");
    for rule in rules {
        log::debug!("{}", rule);
    }

    log::debug!("----- Clauses ------");
    for clause in clauses {
        log::debug!("{:?}", clause);
    }
}

fn debug_print(rules: &[Rule], clauses: &[Clause]) {
    println!("----- Rules ------");
    for rule in rules {
        println!("{}", rule);
    }

    println!("----- Clauses ------");
    for clause in clauses {
        println!("{:?}", clause);
    }
}

fn solve_not_followed_by(clauses: &mut [Clause]) {
    let mut changed = true;
    while changed {
        for i in 0..clauses.len() {
            changed = false;

            let clause = &clauses[i];
            match clause.kind {
                ClauseKind::FollowedBy => {
                    let change = {
                        let sub = &clause.sub[0];
                        let subcl = &clauses[sub.idx];
                        if subcl.kind == ClauseKind::NotFollowedBy {
                            Some(subcl.sub[0].clone())
                        } else {
                            None
                        }
                    };

                    if let Some(sub) = change {
                        let clause = &mut clauses[i];
                        clause.kind = ClauseKind::NotFollowedBy;
                        clause.sub[0] = sub;
                        changed = true;
                    }
                }
                ClauseKind::NotFollowedBy => {
                    let change = {
                        let sub = &clause.sub[0];
                        let subcl = &clauses[sub.idx];
                        if subcl.kind == ClauseKind::NotFollowedBy {
                            Some(subcl.sub[0].clone())
                        } else {
                            None
                        }
                    };

                    if let Some(sub) = change {
                        let clause = &mut clauses[i];
                        clause.kind = ClauseKind::FollowedBy;
                        clause.sub[0] = sub;
                        changed = true;
                    }
                }
                _ => {}
            }
        }
    }
}

fn setup_seed_parents(clauses: &mut [Clause]) {
    for i in 0..clauses.len() {
        let clause = &mut clauses[i];
        let subs = mem::take(&mut clause.sub);

        match &clause.kind {
            ClauseKind::Sequence => {
                for s in &subs {
                    let clause = &mut clauses[s.idx];
                    clause.parents.push(i);

                    if !clause.can_match_zero {
                        break;
                    }
                }
            }
            _ => {
                for s in &subs {
                    let clause = &mut clauses[s.idx];
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

            use ClauseKind::*;
            let new = match &clause.kind {
                Choice => clause.sub.iter().any(|i| (&clauses[i.idx]).can_match_zero),
                CharSequence(s) => s.is_empty(),
                CharRange(a, b) => a > b,
                Nothing => true,
                NotFollowedBy => true,
                _ => clause.sub.iter().all(|i| (&clauses[i.idx]).can_match_zero),
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
                all.remove(s.idx);
            }
        }
        all
    };

    let cycles = find_cycle_head_clauses(&top, rule_starts, &clauses);
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
            if visited.contains(sub.idx) {
                result.insert(sub.idx);
            } else if !finished.contains(sub.idx) {
                detect_clause_cycles_rec(sub.idx, clauses, visited, finished, result);
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
        topo_clauses_rec(sub.idx, clauses, visited, order);
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
