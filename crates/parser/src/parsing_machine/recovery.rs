use crate::grammar::{Rule, Rules};

impl Rules {
    /// Creates a recovery rule for each checkpoint
    pub(crate) fn recovery_rules(&self) {
        // 1. Calculate follow sets for each checkpoint
        // 2. Create &followset rule as a recovery rule
    }

    /// Returns a set of terminals that may match at the start of this rule
    pub fn first_set_of(&self, i: usize) -> Vec<Rule> {
        let ri = &self[i];

        let mut result = vec![];
        let mut seen: Box<[bool]> = vec![false; self.len()].into();
        seen[i] = true;

        first_rec(&ri.rule, self, &mut seen, &mut result);

        result
    }
}

/// Pushes terminals to result and returns whether zero chars can match
fn first_rec(rule: &Rule, rules: &Rules, seen: &mut [bool], result: &mut Vec<Rule>) -> bool {
    use Rule::*;
    let mut can_match_zero = false;

    match rule {
        OneOrMore(r) => {
            can_match_zero |= first_rec(r, rules, seen, result);
        }
        ZeroOrMore(r) | Optional(r) => {
            first_rec(r, rules, seen, result);
            can_match_zero = true;
        }
        Choice(choice_rules) => {
            for rule in choice_rules {
                can_match_zero |= first_rec(rule, rules, seen, result);
            }
        }
        Sequence(seq_rules) => {
            for rule in seq_rules {
                can_match_zero = first_rec(rule, rules, seen, result);
                if !can_match_zero {
                    break;
                }
            }
        }
        ByteSequence(s) => {
            result.push(ByteSequence(vec![s[0]]));
        }
        ByteRange(_, _) | ByteAny | UTF8Range(_, _) => result.push(rule.clone()),
        Ref(j) => {
            if !seen[*j] {
                seen[*j] = true;
                let ri = &rules[*j];
                can_match_zero |= first_rec(&ri.rule, rules, seen, result);
            }
        }
        // TODO what to do with these, they dont consume
        // FollowedBy(r) => {
        //     first_rec(r, rules, seen, result);
        // }
        // NotFollowedBy(r) => {
        //     first_rec(r, rules, seen, result);
        // }
        _ => {}
    }

    can_match_zero
}
