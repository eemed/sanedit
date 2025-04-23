use thiserror::Error;

use crate::grammar::Rule;
use crate::grammar::RuleInfo;
use crate::grammar::Rules;
use crate::Capture;
use crate::Operation;
use crate::ParseError;
use crate::Parser;

pub struct Regex {
    parser: Parser,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, RegexError> {
        let rules = RegexToPEG::convert(pattern)?;
        let parser = Parser::from_rules(rules)?;
        Ok(Regex { parser })
    }

    pub fn is_match<B: AsRef<[u8]>>(&self, bytes: &B) -> bool {
        let bytes = bytes.as_ref();
        match self.parser.parse(bytes) {
            Ok(cap) => {
                println!("OK: {cap:?}");
                true
            }
            Err(_e) => false,
        }
    }
}

struct RegexToPEG<'a> {
    pattern: &'a str,
    parser: Parser,
    regex: Vec<Capture>,
    rules: Vec<RuleInfo>,
    n: usize,
}

impl<'a> RegexToPEG<'a> {
    /// Convert provided regex to PEG
    pub fn convert(pattern: &str) -> Result<Rules, RegexError> {
        let text = include_str!("../pegs/regex.peg");
        let parser = Parser::new(std::io::Cursor::new(text))?;
        let captures = parser.parse(pattern)?;

        println!("Captures: {captures:?}");
        let mut state = RegexToPEG {
            pattern,
            parser,
            regex: captures,
            rules: vec![],
            n: 0,
        };
        let empty = Rule::ByteSequence(vec![]);
        let info = RuleInfo {
            rule: empty.clone(),
            top: true,
            annotations: vec![],
            name: "root".into(),
        };
        state.rules.push(info);
        state.rules[0].rule = state.convert_rec(0, &empty, 1);
        let rules = Rules::new(state.rules.into_boxed_slice());
        println!("Rules:\n{rules}");
        Ok(rules)
    }

    fn convert_rec(&mut self, index: usize, cont: &Rule, depth: usize) -> Rule {
        let cap = &self.regex[index];
        let range = cap.range();
        let text = &self.pattern[range.start as usize..range.end as usize];
        let label = self.parser.label_for(cap.id);

        let children = {
            let mut children = vec![];
            let mut start = range.start;
            let min = std::cmp::min(index + 1, self.regex.len());
            for (i, icap) in self.regex[min..].iter().enumerate() {
                let irange = icap.range();

                // If capture is past the current capture
                if range.end < irange.end {
                    break;
                }

                // Skip inner captures by only considering the first encountered
                if start <= irange.start {
                    start = irange.end;
                    children.push(min + i);
                }
            }
            children
        };
        println!(
            "Enter depth: {depth}, capture: {} / {label} {text:?}: Children: {children:?}",
            cap.id
        );

        match label {
            "escaped" | "char" => {
                // Π(ε, k) = k (1)
                // Π(c, k) = c k (2)
                let mut text = text.as_bytes().to_vec();
                match cont {
                    Rule::ByteSequence(vec) => {
                        text.extend(vec);
                        return Rule::ByteSequence(text);
                    }
                    _ => {
                        let rule = Rule::ByteSequence(text);
                        return Rule::Sequence(vec![rule, cont.clone()]);
                    }
                }
            }
            "sequence" => {
                // Π(e1e2, k) = Π(e1, Π(e2, k)) (3)
                let mut cont = cont.clone();
                for child in children.iter().rev() {
                    cont = self.convert_rec(*child, &cont, depth + 1);
                }
                return cont;
            }
            "alt" => {
                // Distribute continuation to all alternatives
                // Π(e1|e2, k) = Π(e1, k) / Π(e2, k) (4)
                if children.len() == 1 {
                    return self.convert_rec(children[0], &cont, depth + 1);
                }

                let mut choices = vec![];
                for child in children {
                    let rule = self.convert_rec(child, &cont, depth + 1);
                    choices.push(rule);
                }
                return Rule::Choice(choices);
            }
            "zero_or_more" => {
                // e∗ = e e∗ | ε
                if children.len() != 1 {
                    panic!("Zero or more has wrong number of children");
                }

                let self_ref = Rule::Ref(self.rules.len());
                let epsilon = cont.clone();
                let e = self.convert_rec(children[0], &self_ref, depth + 1);
                let rule = Rule::Choice(vec![e, epsilon]);

                let name = format!("{index}-zero-or-more");
                let info = RuleInfo::new(&name, rule);
                self.rules.push(info);
                return self_ref;
            }
            "one_or_more" => {
                // e+ = e e+ | e
                if children.len() != 1 {
                    panic!("One or more has wrong number of children");
                }
                let self_ref = Rule::Ref(self.rules.len());
                let right = self.convert_rec(children[0], cont, depth + 1);
                let left = self.convert_rec(children[0], &self_ref, depth + 1);
                let rule = Rule::Choice(vec![left, right]);

                let name = format!("{index}-one-or-more");
                let info = RuleInfo::new(&name, rule);
                self.rules.push(info);
                return self_ref;
            }
            "optional" => {
                // e? = e | ε
                if children.len() != 1 {
                    panic!("Optional has wrong number of children");
                }
                let e = self.convert_rec(children[0], &cont, depth + 1);
                let epsilon = cont.clone();
                return Rule::Choice(vec![e, epsilon]);
            }
            "group" => {
                if children.len() != 1 {
                    panic!("Group has wrong number of children");
                }
                self.n += 1;
                let cont = Rule::Sequence(vec![Rule::Embed(Operation::CaptureEnd), cont.clone()]);
                let rule = self.convert_rec(children[0], &cont, depth + 1);
                return Rule::Sequence(vec![Rule::Embed(Operation::CaptureBeginMultiEnd(self.n)), rule]);
            }
            _ => {}
        }
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("Failed to parse grammar: {0}")]
    Parsing(#[from] ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_literal() {
        let regex = Regex::new("abc").unwrap();
        assert!(regex.is_match(b"abc"));
        assert!(!regex.is_match(b"ab"));
        assert!(!regex.is_match(b"ab."));
    }

    #[test]
    fn regex_zero_or_more() {
        let regex = Regex::new("ba*").unwrap();
        assert!(!regex.is_match(b""));
        assert!(regex.is_match(b"baaa"));
        assert!(regex.is_match(b"b"));
        assert!(regex.is_match(b"ba"));
        assert!(!regex.is_match(b"aa"));
    }

    #[test]
    fn regex_escaped() {
        let regex = Regex::new("abc\\.").unwrap();
        assert!(!regex.is_match(b"abdc"));
        assert!(regex.is_match(b"abc."));
    }

    #[test]
    fn regex_alt() {
        let regex = Regex::new("ab|abc").unwrap();
        assert!(regex.is_match(b"ab"));
        assert!(regex.is_match(b"abc"));
        assert!(!regex.is_match(b"ac"));
    }

    #[test]
    fn regex_one_or_more() {
        let regex = Regex::new("(a|ab)+c").unwrap();
        assert!(regex.is_match(b"ac"));
        assert!(regex.is_match(b"abc"));

        assert!(!regex.is_match(b""));
        assert!(!regex.is_match(b"bc"));
        assert!(!regex.is_match(b"c"));
    }

    #[test]
    fn regex_optional() {
        let regex = Regex::new("ab?").unwrap();
        assert!(regex.is_match(b"a"));
        assert!(regex.is_match(b"ab"));

        assert!(!regex.is_match(b"ba"));
    }

    #[test]
    fn regex_group() {
        let regex = Regex::new("(a|ab)*c").unwrap();
        assert!(regex.is_match(b"c"));
        assert!(regex.is_match(b"abc"));
        assert!(regex.is_match(b"ac"));
        assert!(regex.is_match(b"aaaabaac"));
        assert!(regex.is_match(b"ababc"));

        assert!(!regex.is_match(b"abdc"));
        assert!(!regex.is_match(b"zz"));
        assert!(!regex.is_match(b"bc"));
    }

    #[test]
    fn regex_capture() {
        let regex = Regex::new("(ab|bd)c").unwrap();
        assert!(regex.is_match(b"abc"));
        assert!(regex.is_match(b"bdc"));

        assert!(!regex.is_match(b"abd"));
        assert!(!regex.is_match(b"acdc"));
    }
}
