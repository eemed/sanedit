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

/// Matches regex using an iterator or all at once
///
/// # Supported
///
/// * character classes \d \D \w \W \s \S \c(A-Z) \f \v \r \n \0
/// * hex \xff
/// * brackets: [^a-z] also backspace matching with [\b]
/// * repetitions x* x? x+ x{2} x{2,} x{2,5}
/// * lazy repetitions x*? x?? x+? x{2}? x{2,}? x{2,5}?
///
/// # Unsupported
///
/// * unicode support
/// * various lookaheads
/// * backreferences
/// * \b word boundaries + \B non word boundary
/// * ^ $  assertions
/// * \p{UnicodeProperty}, \P{UnicodeProperty}
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

    pub fn captures<B: ByteSource>(&self, reader: B) -> CaptureIter<'_, B> {
        self.parser.captures(reader)
    }

    pub fn parse_rules(pattern: &str) -> Result<RegexRules, RegexError> {
        let rules = RegexToPEG::convert(pattern)?;
        Ok(RegexRules(rules))
    }

    pub fn from_rules(rules: RegexRules) -> Result<Regex, RegexError> {
        let parser = Parser::from_rules(rules.0)?;
        Ok(Regex { parser })
    }
}

impl From<Regex> for crate::Parser {
    fn from(value: Regex) -> Self {
        value.parser.into()
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

    fn cc_nword() -> Rule {
        Rule::Choice(vec![
            Rule::ByteRange(u8::MIN, '0' as u8 - 1),
            Rule::ByteRange('9' as u8 + 1, 'A' as u8 - 1),
            Rule::ByteRange('Z' as u8 + 1, '_' as u8 - 1),
            Rule::ByteRange('_' as u8 + 1, 'a' as u8 - 1),
            Rule::ByteRange('z' as u8 + 1, u8::MAX),
        ])
    }

    fn cc_word() -> Rule {
        Rule::Choice(vec![
            Rule::ByteRange('A' as u8, 'Z' as u8),
            Rule::ByteRange('_' as u8, '_' as u8),
            Rule::ByteRange('a' as u8, 'z' as u8),
            Rule::ByteRange('0' as u8, '9' as u8),
        ])
    }

    fn cc_space() -> Rule {
        // [\f\n\r\t\v\u0020\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\ufeff]
        Rule::Choice(vec![
            Rule::ByteRange(0x0c_u8, 0x0c_u8),
            Rule::ByteRange(0x0b_u8, 0x0b_u8),
            Rule::ByteRange(0x09_u8, 0x09_u8),
            Rule::ByteRange(0x0d_u8, 0x0d_u8),
            Rule::ByteRange(0x0a_u8, 0x0a_u8),
            // Rule::UTF8Range('\u{0020}', '\u{0020}'),
            // Rule::UTF8Range('\u{00a0}', '\u{00a0}'),
            // Rule::UTF8Range('\u{1680}', '\u{1680}'),
            // Rule::UTF8Range('\u{2000}', '\u{200a}'),
            // Rule::UTF8Range('\u{2028}', '\u{2029}'),
            // Rule::UTF8Range('\u{202f}', '\u{202f}'),
            // Rule::UTF8Range('\u{205f}', '\u{205f}'),
            // Rule::UTF8Range('\u{3000}', '\u{3000}'),
            // Rule::UTF8Range('\u{feff}', '\u{feff}'),
        ])
    }

    fn cc_nspace() -> Rule {
        // TODO calculate this and replace here
        Rule::Sequence(vec![
            Rule::NotFollowedBy(Self::cc_space().into()),
            Rule::UTF8Range(char::MIN, char::MAX),
        ])
    }

    fn cc_digit() -> Rule {
        Rule::ByteRange('0' as u8, '9' as u8)
    }

    fn cc_ndigit() -> Rule {
        Rule::Choice(vec![
            Rule::ByteRange(u8::MIN, '0' as u8 - 1),
            Rule::ByteRange('9' as u8 + 1, u8::MAX),
        ])
    }

    fn is_lazy(&self, children: &[usize]) -> bool {
        for child in children.iter().rev() {
            let cap = &self.regex[*child];
            let label = self.parser.label_for(cap.id);
            if label == "lazy" {
                return true;
            }
        }

        false
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
        let add_text = |mut bytes: Vec<u8>| match cont {
            Rule::ByteSequence(vec) => {
                bytes.extend(vec);
                return Ok(Rule::ByteSequence(bytes));
            }
            _ => {
                let rule = Rule::ByteSequence(bytes);
                return Ok(Rule::Sequence(vec![rule, cont.clone()]));
            }
        };
        let seq = |rule: Rule| match cont {
            Rule::Sequence(vec) => {
                let mut seq = Vec::with_capacity(vec.len() + 1);
                seq.push(rule);
                seq.extend_from_slice(vec);
                Rule::Sequence(seq)
            }
            _ => Rule::Sequence(vec![rule, cont.clone()]),
        };

        match label {
            "escaped" | "char" => {
                // Π(ε, k) = k (1)
                // Π(c, k) = c k (2)
                let text = text.as_bytes().to_vec();
                add_text(text)
            }
            "any" => Ok(seq(Rule::ByteAny)),
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
                let e = self.convert_rec(children[0], &cont, depth + 1)?;
                let epsilon = cont.clone();
                let choices = if self.is_lazy(&children) {
                    vec![epsilon, e]
                } else {
                    vec![e, epsilon]
                };
                return Ok(Rule::Choice(choices));
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
            "counted_rep" => Ok(self.convert_counted_rep(children, cont, depth)?),
            "hex_value" => {
                let byte =
                    u8::from_str_radix(text, 16).map_err(|_e| RegexError::InvalidHexValue)?;
                add_text(vec![byte])
            }
            "brackets" => Ok(seq(self.convert_brackets(children)?)),
            "cc_control" => add_text(vec![(text.as_bytes()[1] as u8 - 'A' as u8) + 1]),
            "cc_null" => add_text(vec![0x00_u8]),
            "cc_ff" => add_text(vec![0x0c_u8]),
            "cc_vtab" => add_text(vec![0x0b_u8]),
            "cc_tab" => add_text(vec![0x09_u8]),
            "cc_cr" => add_text(vec![0x0d_u8]),
            "cc_newline" => add_text(vec![0x0a_u8]),
            "cc_nws" => Ok(seq(Self::cc_nspace())),
            "cc_ws" => Ok(seq(Self::cc_space())),
            "cc_word" => Ok(seq(Self::cc_word())),
            "cc_nword" => Ok(seq(Self::cc_nword())),
            "cc_digit" => Ok(seq(Self::cc_digit())),
            "cc_ndigit" => Ok(seq(Self::cc_ndigit())),
            // "codepoint_digits" => Ok(seq(self.convert_codepoints(cap)?)),
            _ => return Err(RegexError::InvalidPattern),
        }
    }

    fn convert_counted_rep(
        &mut self,
        children: Vec<usize>,
        cont: &Rule,
        depth: usize,
    ) -> Result<Rule, RegexError> {
        let mut lazy = false;
        let mut start = 0_usize;
        let mut end: Option<usize> = None;
        let mut is_range = false;

        for child in &children {
            let ccap = &self.regex[*child];
            let crange = ccap.range();
            let text = &self.pattern[crange.start as usize..crange.end as usize];
            let label = self.parser.label_for(ccap.id);

            println!("{label:?} -> {text:?}");
            match label {
                "lazy" => lazy = true,
                "counted_from" => start = text.parse().map_err(|_| RegexError::InvalidRepCount)?,
                "counted_to" => end = Some(text.parse().map_err(|_| RegexError::InvalidRepCount)?),
                "counted_sep" => is_range = true,
                _ => {}
            }
        }

        let empty = Rule::ByteSequence(vec![]);
        let e = self.convert_rec(children[0], &empty, depth + 1)?;

        println!("start: {start:?}, end: {end:?}, is_range: {is_range:?}, cont: {cont:?}");

        // Does lazy even do anything here?
        // patt{x} (?)
        if !is_range {
            let mut repeat = Vec::with_capacity(start);
            for _ in 0..start {
                repeat.push(e.clone());
            }
            repeat.push(cont.clone());
            return Ok(Rule::Sequence(repeat));
        }

        match end {
            Some(end) => {
                // patt{x,y} (?)
                if start >= end {
                    return Err(RegexError::InvalidRepCount);
                }
                let mut repeat = Vec::with_capacity(start);
                for _ in 0..start {
                    repeat.push(e.clone());
                }

                if lazy {
                    let mut rule = e.clone();
                    for _ in start..end {
                        rule = Rule::Choice(vec![cont.clone(), rule]);
                    }
                    repeat.push(rule);
                    return Ok(Rule::Choice(repeat));
                } else {
                    for _ in start..end {
                        repeat.push(Rule::Optional(e.clone().into()));
                    }
                    repeat.push(cont.clone());
                    return Ok(Rule::Sequence(repeat));
                }
            }
            None => {
                // patt{x,} (?)
                let mut repeat = Vec::with_capacity(start);
                for _ in 0..start {
                    repeat.push(e.clone());
                }

                todo!()
            }
        }
    }

    // fn convert_codepoints(&self, cap: &Capture) -> Result<Rule, RegexError> {
    //     let range = cap.range();
    //     let text = &self.pattern[range.start as usize..range.end as usize];
    //     let cp = u32::from_str_radix(text, 16).map_err(|_e| RegexError::InvalidCodepointValue)?;
    //     let ch = char::from_u32(cp).ok_or(RegexError::InvalidCodepointValue)?;
    //     Ok(Rule::UTF8Range(ch, ch))
    // }

    fn convert_brackets(&self, children: Vec<usize>) -> Result<Rule, RegexError> {
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
                "cc_control" => {}
                "cc_null" => {}
                "cc_ff" => {}
                "cc_vtab" => {}
                "cc_tab" => {}
                "cc_cr" => {}
                "cc_newline" => {}
                "cc_nws" => {}
                "cc_ws" => {}
                "cc_word" => {}
                "cc_nword" => {}
                "cc_digit" => {}
                "cc_ndigit" => {}
                _ => return Err(RegexError::InvalidPattern),
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
        Ok(choice)
    }

    fn convert_bracket_range(&self, index: usize) -> Result<Range<u32>, RegexError> {
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

    #[error("Invalid repetiontion count")]
    InvalidRepCount,

    #[error("Invalid hex value")]
    InvalidHexValue,

    // #[error("Invalid codepoint value")]
    // InvalidCodepointValue,
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

    #[test]
    fn regex_digit() {
        let regex = Regex::new(r"\d+").unwrap();
        assert!(!regex.is_match(b"perkele"));
        assert!(!regex.is_match(b"f"));

        assert!(regex.is_match(b"123"));
        assert!(regex.is_match(b"0"));
        assert!(regex.is_match(b"9"));
        assert!(regex.is_match(b"1234567890"));
    }

    #[test]
    fn regex_ndigit() {
        let regex = Regex::new(r"\D+").unwrap();
        assert!(regex.is_match(b"perkele"));
        assert!(regex.is_match(b"f"));

        assert!(!regex.is_match(b"123"));
        assert!(!regex.is_match(b"0"));
        assert!(!regex.is_match(b"9"));
        assert!(!regex.is_match(b"1234567890"));
    }

    #[test]
    fn regex_counted_rep() {
        let regex = Regex::new(r"\d{3}.\d{3}").unwrap();
        assert!(regex.is_match(b"222.222"));
        assert!(regex.is_match(b"192.168"));

        assert!(!regex.is_match(b"123"));
        assert!(!regex.is_match(b"0"));
        assert!(!regex.is_match(b"9"));
        assert!(!regex.is_match(b"123..213"));
    }

    #[test]
    fn regex_lazy_optional() {
        let regex = Regex::new(r"a??ab").unwrap();
        println!("{:?}", regex.parser.program());
        assert!(regex.is_match(b"ab"));
        assert!(regex.is_match(b"aab"));
        assert!(regex.is_match(b"xxab"));
        assert!(regex.is_match(b"xxaab"));

        assert!(!regex.is_match(b"xxxx"));
        assert!(!regex.is_match(b"a"));
    }

    // TODO test out all lazy operators, character classes, [\b] match
}
