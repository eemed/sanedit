use std::ops::Range;
use std::sync::Arc;
use std::sync::OnceLock;

use sanedit_utils::ranges::OverlappingRanges;
use sanedit_utils::sorted_vec::SortedVec;
use thiserror::Error;

use crate::grammar::Rule;
use crate::grammar::RuleInfo;
use crate::grammar::Rules;
use crate::ByteSource;
use crate::Capture;
use crate::CaptureIter;
use crate::Operation;
use crate::ParseError;
use crate::ParserKind as Parser;

pub struct RegexRules(pub(crate) Rules);

impl std::fmt::Display for RegexRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for RegexRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

fn regex_parser() -> &'static Parser {
    static PARSER: OnceLock<Arc<Parser>> = OnceLock::new();
    let parser = PARSER.get_or_init(|| {
        let text = include_str!("../pegs/regex.peg");
        let parser = Parser::new(std::io::Cursor::new(text)).unwrap();
        Arc::new(parser)
    });
    parser.as_ref()
}

#[derive(Debug)]
pub struct Regex {
    parser: Parser,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, RegexError> {
        let rules = RegexToPEG::convert(pattern)?;
        let parser = Parser::from_rules_unanchored(rules)?;
        Ok(Regex { parser })
    }


    pub fn is_match<B: AsRef<[u8]>>(&self, bytes: &B) -> bool {
        let bytes = bytes.as_ref();
        let captures = self.parser.parse(bytes);
        captures.is_ok()
    }

    pub fn captures<B: ByteSource>(&self, reader: B) -> CaptureIter<B> {
        self.parser.captures(reader)
    }
}

impl From<Regex> for Parser {
    fn from(value: Regex) -> Self {
        value.parser
    }
}

struct RegexToPEG<'a> {
    pattern: &'a str,
    parser: &'static Parser,
    regex: SortedVec<Capture>,
    rules: Vec<RuleInfo>,
    n: usize,
}

impl<'a> RegexToPEG<'a> {
    /// Convert provided regex to PEG
    pub fn convert(pattern: &str) -> Result<Rules, RegexError> {
        let parser = regex_parser();
        let pattern = format!("({pattern})");
        let captures: SortedVec<Capture> = parser.parse(pattern.as_str())?.into();

        let mut state = RegexToPEG {
            pattern: pattern.as_str(),
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
        state.rules[0].rule = state.convert_rec(0, &empty, 1)?;
        let rules = Rules::new(state.rules.into_boxed_slice());
        Ok(rules)
    }

    fn convert_rec(&mut self, index: usize, cont: &Rule, depth: usize) -> Result<Rule, RegexError> {
        if index >= self.regex.len() {
            return Err(RegexError::InvalidPattern);
        }
        let cap = &self.regex[index];
        let range = cap.range();
        let text = &self.pattern[range.start as usize..range.end as usize];
        let label = self.parser.label_for(cap.id);

        let children = {
            let mut children = vec![];
            let mut start = range.start;
            let min = std::cmp::min(index + 1, self.regex.len());
            for (i, icap) in self.regex.iter().skip(min).enumerate() {
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
        let is_full_match = cap.start == 0 && cap.end == self.pattern.len() as u64;

        match label {
            "escaped" | "char" => {
                // Π(ε, k) = k (1)
                // Π(c, k) = c k (2)
                let mut text = text.as_bytes().to_vec();
                match cont {
                    Rule::ByteSequence(vec) => {
                        text.extend(vec);
                        return Ok(Rule::ByteSequence(text));
                    }
                    _ => {
                        let rule = Rule::ByteSequence(text);
                        return Ok(Rule::Sequence(vec![rule, cont.clone()]));
                    }
                }
            }
            "any" => {
                let rule = Rule::ByteAny;
                return Ok(Rule::Sequence(vec![rule, cont.clone()]));
            }
            "sequence" => {
                // Π(e1e2, k) = Π(e1, Π(e2, k)) (3)
                let mut cont = cont.clone();
                for child in children.iter().rev() {
                    cont = self.convert_rec(*child, &cont, depth + 1)?;
                }
                return Ok(cont);
            }
            "alt" => {
                // Distribute continuation to all alternatives
                // Π(e1|e2, k) = Π(e1, k) / Π(e2, k) (4)
                if children.len() == 1 {
                    return self.convert_rec(children[0], &cont, depth + 1);
                }

                let mut choices = vec![];
                for child in children {
                    let rule = self.convert_rec(child, &cont, depth + 1)?;
                    choices.push(rule);
                }
                return Ok(Rule::Choice(choices));
            }
            "zero_or_more" => {
                // e∗ = e e∗ | ε
                if children.len() != 1 {
                    panic!("Zero or more has wrong number of children");
                }

                let pos = self.rules.len();
                let self_ref = Rule::Ref(pos);
                let name = format!("{index}-zero-or-more");
                self.rules.push(RuleInfo::new(&name, Rule::ByteAny));

                let epsilon = cont.clone();
                let e = self.convert_rec(children[0], &self_ref, depth + 1)?;
                let rule = Rule::Choice(vec![e, epsilon]);

                self.rules[pos].rule = rule;
                return Ok(self_ref);
            }
            "one_or_more" => {
                // e+ = e e+ | e
                if children.len() != 1 {
                    panic!("One or more has wrong number of children");
                }
                let pos = self.rules.len();
                let self_ref = Rule::Ref(pos);
                let name = format!("{index}-one-or-more");
                self.rules.push(RuleInfo::new(&name, Rule::ByteAny));

                let right = self.convert_rec(children[0], cont, depth + 1)?;
                let left = self.convert_rec(children[0], &self_ref, depth + 1)?;
                let rule = Rule::Choice(vec![left, right]);

                self.rules[pos].rule = rule;
                return Ok(self_ref);
            }
            "optional" => {
                // e? = e | ε
                if children.len() != 1 {
                    panic!("Optional has wrong number of children");
                }
                let e = self.convert_rec(children[0], &cont, depth + 1)?;
                let epsilon = cont.clone();
                return Ok(Rule::Choice(vec![e, epsilon]));
            }
            "group" => {
                if children.len() != 1 {
                    panic!("Group has wrong number of children");
                }

                let cont = Rule::Sequence(vec![Rule::Embed(Operation::CaptureEnd), cont.clone()]);
                let rule = self.convert_rec(children[0], &cont, depth + 1)?;
                self.n += 1;
                let n = if is_full_match { 0 } else { self.n };
                return Ok(Rule::Sequence(vec![
                    Rule::Embed(Operation::CaptureBegin(n)),
                    rule,
                ]));
            }
            "hex_value" => {
                let byte =
                    u8::from_str_radix(text, 16).map_err(|_e| RegexError::InvalidHexValue)?;
                match cont {
                    Rule::ByteSequence(vec) => {
                        let mut bytes = vec![byte];
                        bytes.extend(vec);
                        return Ok(Rule::ByteSequence(bytes));
                    }
                    _ => {
                        let rule = Rule::ByteSequence(vec![byte]);
                        return Ok(Rule::Sequence(vec![rule, cont.clone()]));
                    }
                }
            }
            "brackets" => {
                let mut ranges = OverlappingRanges::new();
                let mut negative = false;
                for child in children {
                    let ccap = &self.regex[child];
                    let crange = ccap.range();
                    let clabel = self.parser.label_for(ccap.id());
                    let text = &self.pattern[crange.start as usize..crange.end as usize];

                    match clabel {
                        "range" => {
                            let range = self.convert_bracket_range(child)?;
                            ranges.add(range);
                        }
                        "hex_value" => {
                            let byte = u8::from_str_radix(text, 16)
                                .map_err(|_e| RegexError::InvalidHexValue)?
                                as u32;
                            ranges.add(byte..byte + 1);
                        }
                        "byte" => {
                            let byte = text.as_bytes()[0] as u32;
                            ranges.add(byte..byte + 1);
                        }
                        "neg" => {
                            negative = true;
                        }
                        p => unreachable!("Invalid label in brackets {p}"),
                    }
                }

                if negative {
                    ranges.invert(u8::MIN as u32..u8::MAX as u32 + 1)
                }

                let mut choices = vec![];

                for range in ranges.iter() {
                    choices.push(Rule::ByteRange(range.start as u8, (range.end - 1) as u8))
                }

                let choice = if choices.len() == 1 {
                    choices.pop().unwrap()
                } else {
                    Rule::Choice(choices)
                };

                Ok(Rule::Sequence(vec![choice, cont.clone()]))
            }
            p => unreachable!("Invalid label {p}"),
        }
    }

    fn convert_bracket_range(&mut self, index: usize) -> Result<Range<u32>, RegexError> {
        if index >= self.regex.len() {
            return Err(RegexError::InvalidPattern);
        }
        let cap = &self.regex[index];
        let range = cap.range();

        let children = {
            let mut children = vec![];
            let mut start = range.start;
            let min = std::cmp::min(index + 1, self.regex.len());
            for (i, icap) in self.regex.iter().skip(min).enumerate() {
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

        if children.len() != 2 {
            return Err(RegexError::InvalidRange);
        }

        let mut result = [0u32; 2];

        for (i, child) in children.iter().enumerate() {
            let cap = &self.regex[*child];
            let range = cap.range();
            let text = &self.pattern[range.start as usize..range.end as usize];
            let label = self.parser.label_for(cap.id);

            match label {
                "hex_value" => {
                    let byte = u8::from_str_radix(text, 16)
                        .map_err(|_e| RegexError::InvalidHexValue)?
                        as u32;
                    result[i] = byte;
                }
                "byte" => {
                    let byte = text.as_bytes()[0] as u32;
                    result[i] = byte;
                }
                _ => {}
            }
        }

        return Ok(result[0]..result[1] + 1);
    }
}

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("Failed to parse grammar: {0}")]
    Parsing(#[from] ParseError),

    #[error("Invalid regex pattern")]
    InvalidPattern,

    #[error("Invalid hex value")]
    InvalidHexValue,

    #[error("Invalid bracket range")]
    InvalidRange,
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
        assert!(regex.is_match(b"ba"));
    }

    #[test]
    fn regex_group() {
        let regex = Regex::new("(a|ab)*c").unwrap();
        assert!(regex.is_match(b"c"));
        assert!(regex.is_match(b"abc"));
        assert!(regex.is_match(b"ac"));
        assert!(regex.is_match(b"aaaabaac"));
        assert!(regex.is_match(b"ababc"));
        assert!(regex.is_match(b"xxxac"));
        assert!(regex.is_match(b"bc"));

        assert!(!regex.is_match(b"abd"));
        assert!(!regex.is_match(b"zz"));
    }

    #[test]
    fn regex_capture() {
        let regex = Regex::new("(ab|bd)c").unwrap();
        assert!(regex.is_match(b"abc"));
        assert!(regex.is_match(b"bdc"));

        assert!(!regex.is_match(b"abd"));
        assert!(!regex.is_match(b"acdc"));
    }

    #[test]
    fn regex_any() {
        let regex = Regex::new(".*").unwrap();
        assert!(regex.is_match(b"a"));
        assert!(regex.is_match(b"ab"));
        assert!(regex.is_match(b"ab\ndc"));
    }

    #[test]
    fn regex_class() {
        let regex = Regex::new("[a-z0-9C]+").unwrap();
        assert!(regex.is_match(b"AAAAAzooom"));
        assert!(regex.is_match(b"fooCbar"));
        assert!(regex.is_match(b"f"));
        assert!(regex.is_match(b"AAAAAbAAAAA"));
        assert!(regex.is_match(b"AAACAAA"));

        assert!(!regex.is_match(b"AAAAAA"));
        assert!(!regex.is_match(b""));
    }

    #[test]
    fn regex_class_neg() {
        let regex = Regex::new("[^\x41-\x43\x50]+").unwrap();
        assert!(regex.is_match(b"perkele"));
        assert!(regex.is_match(b"hellowarld"));
        assert!(regex.is_match(b"f"));

        assert!(!regex.is_match(b""));
        assert!(!regex.is_match(b"A"));
        assert!(!regex.is_match(b"B"));
        assert!(!regex.is_match(b"C"));
        assert!(!regex.is_match(b"P"));
        assert!(!regex.is_match(b"APBC"));
    }
}
