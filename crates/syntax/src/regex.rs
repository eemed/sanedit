use std::ops::Range;
use std::sync::Arc;
use std::sync::OnceLock;

use sanedit_utils::ranges::OverlappingRanges;
use sanedit_utils::sorted_vec::SortedVec;
use thiserror::Error;

use crate::grammar::Rule;
use crate::grammar::RuleInfo;
use crate::grammar::Rules;
use crate::source::Source;
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
/// * unicodepoints \u{fefe}
///
/// # Unsupported
///
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
        let mut bytes = bytes.as_ref();
        let captures = self.parser.parse(&mut bytes);
        captures.is_ok()
    }

    pub fn captures<'b, S: Source>(&self, reader: &'b mut S) -> CaptureIter<'_, 'b, S> {
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
        let captures: SortedVec<Capture> = parser.parse(&mut pattern.as_str())?.into();

        let mut state = RegexToPEG {
            pattern: pattern.as_str(),
            parser,
            regex: captures,
            rules: vec![],
            n: 0,
        };
        let empty = Rule::ByteSequence(vec![]);
        let mut info = RuleInfo::new("root".into(), empty.clone());
        info.top = true;
        state.rules.push(info);
        state.rules[0].rule = state.convert_rec(0, &empty, 1)?;
        let rules = Rules::new(state.rules.into_boxed_slice());
        Ok(rules)
    }

    fn cc_nword() -> Rule {
        Rule::Choice(vec![
            Rule::ByteRange(u8::MIN, b'0' - 1),
            Rule::ByteRange(b'9' + 1, b'A' - 1),
            Rule::ByteRange(b'Z' + 1, b'_' - 1),
            Rule::ByteRange(b'_' + 1, b'a' - 1),
            Rule::ByteRange(b'z' + 1, u8::MAX),
        ])
    }

    fn cc_word() -> Rule {
        Rule::Choice(vec![
            Rule::ByteRange(b'A', b'Z'),
            Rule::ByteRange(b'_', b'_'),
            Rule::ByteRange(b'a', b'z'),
            Rule::ByteRange(b'0', b'9'),
        ])
    }

    fn cc_space() -> Rule {
        // [\f\n\r\t\v\u0020\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\ufeff]
        Rule::Choice(vec![
            Rule::ByteRange(0x09_u8, 0x0d_u8),
            Rule::ByteRange(0x20_u8, 0x20_u8),
            // Rule::ByteRange(0x0b_u8, 0x0b_u8),
            // Rule::ByteRange(0x09_u8, 0x09_u8),
            // Rule::ByteRange(0x0d_u8, 0x0d_u8),
            // Rule::ByteRange(0x0a_u8, 0x0a_u8),
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
        Rule::Choice(vec![
            Rule::ByteRange(0x0_u8, 0x08_u8),
            Rule::ByteRange(0x0e_u8, 0x1f_u8),
            Rule::ByteRange(0x21_u8, 0xff_u8),
        ])
    }

    fn cc_digit() -> Rule {
        Rule::ByteRange(b'0', b'9')
    }

    fn cc_ndigit() -> Rule {
        Rule::Choice(vec![
            Rule::ByteRange(u8::MIN, b'0' - 1),
            Rule::ByteRange(b'9' + 1, u8::MAX),
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
                Ok(Rule::ByteSequence(bytes))
            }
            _ => {
                let rule = Rule::ByteSequence(bytes);
                Ok(Rule::Sequence(vec![rule, cont.clone()]))
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
                Ok(cont)
            }
            "alt" => {
                // Distribute continuation to all alternatives
                // Π(e1|e2, k) = Π(e1, k) / Π(e2, k) (4)
                if children.len() == 1 {
                    return self.convert_rec(children[0], cont, depth + 1);
                }

                let mut choices = vec![];
                for child in children {
                    let rule = self.convert_rec(child, cont, depth + 1)?;
                    choices.push(rule);
                }
                Ok(Rule::Choice(choices))
            }
            "zero_or_more" => {
                // e∗ = e e∗ | ε
                let pos = self.rules.len();
                let self_ref = Rule::Ref(pos);
                self.rules.push(RuleInfo::new(
                    format!("{index}-zero-or-more"),
                    Rule::ByteAny,
                ));

                let epsilon = cont.clone();
                let e = self.convert_rec(children[0], &self_ref, depth + 1)?;
                let choices = if self.is_lazy(&children) {
                    vec![epsilon, e]
                } else {
                    vec![e, epsilon]
                };
                let rule = Rule::Choice(choices);

                self.rules[pos].rule = rule;
                Ok(self_ref)
            }
            "one_or_more" => {
                // e+ = e e+ | e
                // XXX e+ = e e*
                let pos = self.rules.len();
                let self_ref = Rule::Ref(pos);
                self.rules
                    .push(RuleInfo::new(format!("{index}-one-or-more"), Rule::ByteAny));

                let right = self.convert_rec(children[0], cont, depth + 1)?;
                let left = self.convert_rec(children[0], &self_ref, depth + 1)?;
                let choices = if self.is_lazy(&children) {
                    vec![right, left]
                } else {
                    vec![left, right]
                };
                let rule = Rule::Choice(choices);

                self.rules[pos].rule = rule;
                Ok(self_ref)
            }
            "optional" => {
                // e? = e | ε
                let e = self.convert_rec(children[0], cont, depth + 1)?;
                let epsilon = cont.clone();
                let choices = if self.is_lazy(&children) {
                    vec![epsilon, e]
                } else {
                    vec![e, epsilon]
                };
                Ok(Rule::Choice(choices))
            }
            "group" => {
                if children.len() != 1 {
                    panic!("Group has wrong number of children");
                }

                let cont = Rule::Sequence(vec![Rule::Embed(Operation::CaptureEnd), cont.clone()]);
                let rule = self.convert_rec(children[0], &cont, depth + 1)?;
                self.n += 1;
                let n = if is_full_match { 0 } else { self.n };
                Ok(Rule::Sequence(vec![
                    Rule::Embed(Operation::CaptureBegin(n)),
                    rule,
                ]))
            }
            "counted_rep" => Ok(self.convert_counted_rep(children, cont, depth, index)?),
            "hex_value" => {
                let byte =
                    u8::from_str_radix(text, 16).map_err(|_e| RegexError::InvalidHexValue)?;
                add_text(vec![byte])
            }
            "brackets" => Ok(seq(self.convert_brackets(children)?)),
            "cc_control" => add_text(vec![(text.as_bytes()[1] - b'A') + 1]),
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
            "codepoint_digits" => Ok(seq(self.convert_codepoint_to_rule(cap)?)),
            _ => Err(RegexError::InvalidPattern),
        }
    }

    fn convert_counted_rep(
        &mut self,
        children: Vec<usize>,
        cont: &Rule,
        depth: usize,
        index: usize,
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

        // Does lazy even do anything here?
        // patt{x} (?)
        if !is_range {
            let mut repeat = Vec::with_capacity(start + 1);
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
                let mut repeat = vec![];
                for _ in 0..start {
                    repeat.push(e.clone());
                }

                if lazy {
                    let mut rule = e.clone();
                    for _ in start..end {
                        rule = Rule::Choice(vec![cont.clone(), rule]);
                    }
                    repeat.push(rule);
                    Ok(Rule::Sequence(repeat))
                } else {
                    for _ in start..end {
                        repeat.push(Rule::Optional(e.clone().into()));
                    }
                    repeat.push(cont.clone());
                    Ok(Rule::Sequence(repeat))
                }
            }
            None => {
                // patt{x,} (?)
                let mut repeat = vec![];
                for _ in 0..start {
                    repeat.push(e.clone());
                }

                // Zero or more rest
                let pos = self.rules.len();
                let self_ref = Rule::Ref(pos);
                self.rules
                    .push(RuleInfo::new(format!("{index}-counted-rep"), Rule::ByteAny));

                let epsilon = cont.clone();
                let e = self.convert_rec(children[0], &self_ref, depth + 1)?;
                let choices = if lazy {
                    vec![epsilon, e]
                } else {
                    vec![e, epsilon]
                };
                let rule = Rule::Choice(choices);

                self.rules[pos].rule = rule;
                repeat.push(self_ref);
                Ok(Rule::Sequence(repeat))
            }
        }
    }

    fn convert_codepoint(&self, cap: &Capture) -> Result<u32, RegexError> {
        let range = cap.range();
        let text = &self.pattern[range.start as usize..range.end as usize];
        u32::from_str_radix(text, 16).map_err(|_e| RegexError::InvalidCodepointValue)
    }

    fn convert_codepoint_to_rule(&self, cap: &Capture) -> Result<Rule, RegexError> {
        let cp = self.convert_codepoint(cap)?;
        let ch = char::from_u32(cp).ok_or(RegexError::InvalidCodepointValue)?;
        Ok(Rule::UTF8Range(ch, ch))
    }

    fn collect_ranges_and_insert(ranges: &mut OverlappingRanges<u32>, rule: Rule) {
        match rule {
            Rule::Choice(choices) => {
                for choice in choices {
                    match choice {
                        Rule::ByteRange(a, b) => ranges.add(a as u32..b as u32 + 1),
                        Rule::UTF8Range(a, b) => ranges.add(a as u32..b as u32 + 1),
                        _ => {}
                    }
                }
            }
            Rule::ByteRange(a, b) => ranges.add(a as u32..b as u32 + 1),
            Rule::UTF8Range(a, b) => ranges.add(a as u32..b as u32 + 1),
            _ => {}
        }
    }

    fn convert_brackets(&self, children: Vec<usize>) -> Result<Rule, RegexError> {
        let mut ranges = OverlappingRanges::new();
        let mut negative = false;
        let mut unicode = false;
        for child in children {
            let ccap = &self.regex[child];
            let crange = ccap.range();
            let clabel = self.parser.label_for(ccap.id());
            let text = &self.pattern[crange.start as usize..crange.end as usize];

            match clabel {
                "range" => {
                    let range = self.convert_bracket_range(child)?;
                    if range.end > u8::MAX as u32 {
                        unicode = true;
                    }
                    ranges.add(range);
                }
                "hex_value" => {
                    let byte = u8::from_str_radix(text, 16)
                        .map_err(|_e| RegexError::InvalidHexValue)?
                        as u32;
                    ranges.add(byte..byte + 1);
                }
                "any_utf8" => {
                    let ch = text
                        .chars()
                        .next()
                        .ok_or(RegexError::InvalidCodepointValue)?
                        as u32;
                    ranges.add(ch..ch + 1);
                    let is_utf8 = ch > u8::MAX as u32;
                    unicode |= is_utf8;
                }
                "neg" => negative = true,
                "cc_control" => {
                    let ctrl = (text.as_bytes()[1] as u32 - 'A' as u32) + 1;
                    ranges.add(ctrl..ctrl + 1);
                }
                "cc_null" => ranges.add(0x00_u32..1),
                "cc_ff" => ranges.add(0x0c_u32..0x0c_u32 + 1),
                "cc_vtab" => ranges.add(0x0b_u32..0x0b_u32 + 1),
                "cc_tab" => ranges.add(0x09_u32..0x09_u32 + 1),
                "cc_cr" => ranges.add(0x0d_u32..0x0d_u32 + 1),
                "cc_newline" => ranges.add(0x0a_u32..0x0a_u32 + 1),
                "backspace" => ranges.add(0x08_u32..0x08_u32 + 1),
                "cc_nws" => Self::collect_ranges_and_insert(&mut ranges, Self::cc_nspace()),
                "cc_ws" => Self::collect_ranges_and_insert(&mut ranges, Self::cc_space()),
                "cc_word" => Self::collect_ranges_and_insert(&mut ranges, Self::cc_word()),
                "cc_nword" => Self::collect_ranges_and_insert(&mut ranges, Self::cc_nword()),
                "cc_digit" => Self::collect_ranges_and_insert(&mut ranges, Self::cc_digit()),
                "cc_ndigit" => Self::collect_ranges_and_insert(&mut ranges, Self::cc_ndigit()),
                "escaped" => {
                    let byte = text.as_bytes()[0] as u32;
                    ranges.add(byte..byte + 1);
                }
                "codepoint_digits" => {
                    unicode = true;
                    let cp = self.convert_codepoint(ccap)?;
                    ranges.add(cp..cp + 1);
                }
                _ => return Err(RegexError::InvalidPattern),
            }
        }

        if !unicode {
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
            return Ok(choice);
        }

        if negative {
            ranges.invert('\u{0000}' as u32..'\u{10ffff}' as u32 + 1)
        }
        let mut choices = vec![];

        for range in ranges.iter() {
            let fch = char::from_u32(range.start).ok_or(RegexError::InvalidCodepointValue)?;
            let sch = char::from_u32(range.end - 1).ok_or(RegexError::InvalidCodepointValue)?;
            choices.push(Rule::UTF8Range(fch, sch));
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
                "any_utf8" => {
                    let ch = text
                        .chars()
                        .next()
                        .ok_or(RegexError::InvalidCodepointValue)?
                        as u32;
                    result[i] = ch;
                }
                "escaped" => {
                    let byte = u8::from_str_radix(text, 16)
                        .map_err(|_e| RegexError::InvalidHexValue)?
                        as u32;
                    result[i] = byte;
                }
                "codepoint_digits" => result[i] = self.convert_codepoint(cap)?,
                "cc_control" => {
                    let ctrl = (text.as_bytes()[1] as u32 - 'A' as u32) + 1;
                    result[i] = ctrl;
                }
                _ => return Err(RegexError::InvalidPattern),
            }
        }

        Ok(result[0]..result[1] + 1)
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

    #[error("Invalid codepoint value")]
    InvalidCodepointValue,

    #[error("Invalid bracket range")]
    InvalidRange,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap_len(cap: &Capture) -> u64 {
        cap.end - cap.start
    }

    fn is_total_match(regex: &Regex, mut bytes: &[u8]) {
        let caps = regex.captures(&mut bytes).next().expect("No match");
        assert_eq!(cap_len(&caps[0]), bytes.len() as u64)
    }

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
        let regex = Regex::new(r"[^\x41-\x43\x50]+").unwrap();
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
    fn regex_word() {
        let regex = Regex::new(r"\w+").unwrap();
        is_total_match(&regex, b"aaaaa");
        is_total_match(&regex, b"aNOTHer123");
        assert!(!regex.is_match(b"$"));
        assert!(!regex.is_match(b"("));
        assert!(!regex.is_match(b"!"));
    }

    #[test]
    fn regex_nword() {
        let regex = Regex::new(r"\W+").unwrap();
        assert!(!regex.is_match(b"hello_world"));
        assert!(!regex.is_match(b"another"));
        assert!(!regex.is_match(b"0"));
        assert!(regex.is_match(b"$"));
        assert!(regex.is_match(b"("));
        assert!(regex.is_match(b"!"));
    }

    #[test]
    fn regex_space() {
        let regex = Regex::new(r"\s+").unwrap();
        is_total_match(&regex, b"   ");
        is_total_match(&regex, b"\t");
        // is_total_match(&regex, &"\u{2028}".as_bytes());
        assert!(!regex.is_match(b"a"));
        assert!(!regex.is_match(b"b"));
        assert!(!regex.is_match(b"123"));
    }

    #[test]
    fn regex_nspace() {
        let regex = Regex::new(r"\S+").unwrap();
        assert!(!regex.is_match(b"   "));
        assert!(!regex.is_match(b"\t"));
        assert!(regex.is_match(b"a"));
        assert!(regex.is_match(b"b"));
        assert!(regex.is_match(b"123"));
    }

    #[test]
    fn regex_nspace_group() {
        let regex = Regex::new(r"[\S ]+").unwrap();
        is_total_match(&regex, b"what the fuck");
        assert!(!regex.is_match(b"\t"));
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
    fn regex_counted_rep_inf() {
        let regex = Regex::new(r"\d{3,}").unwrap();
        assert!(regex.is_match(b"222"));
        assert!(regex.is_match(b"192"));
        assert!(regex.is_match(b"192168111"));
        assert!(regex.is_match(b"1234567890"));

        assert!(!regex.is_match(b"13"));
        assert!(!regex.is_match(b"0"));
        assert!(!regex.is_match(b"9"));
    }

    #[test]
    fn regex_counted_rep_capped() {
        let regex = Regex::new(r"\d{3,5}").unwrap();
        assert!(regex.is_match(b"222"));
        assert!(regex.is_match(b"1924"));
        assert!(regex.is_match(b"19245"));
        let cap = regex
            .captures(&mut b"1111111111111")
            .next()
            .expect("Did not match");
        assert_eq!(cap_len(&cap[0]), 5);

        assert!(!regex.is_match(b"13"));
        assert!(!regex.is_match(b"0"));
        assert!(!regex.is_match(b"9"));
    }

    #[test]
    fn regex_hex() {
        let regex = Regex::new(r"\x41").unwrap();
        assert!(regex.is_match(b"A"));

        assert!(!regex.is_match(b"B"));
        assert!(!regex.is_match(b"xxxx"));
        assert!(!regex.is_match(b"a"));
    }

    #[test]
    fn regex_lazy_optional() {
        let regex = Regex::new(r"a??ab").unwrap();
        assert!(regex.is_match(b"ab"));
        assert!(regex.is_match(b"aab"));
        assert!(regex.is_match(b"xxab"));
        assert!(regex.is_match(b"xxaab"));

        assert!(!regex.is_match(b"xxxx"));
        assert!(!regex.is_match(b"a"));
    }

    #[test]
    fn regex_lazy_zero_or_more() {
        let regex = Regex::new(r"ab*?c").unwrap();
        assert!(regex.is_match(b"ac"));
        assert!(regex.is_match(b"abc"));
        assert!(regex.is_match(b"abbbc"));

        let regex = Regex::new(r"a[bc]*?").unwrap();
        let cap = regex.captures(&mut b"abc").next().expect("Did not match");
        assert_eq!(cap_len(&cap[0]), 1);
        assert!(regex.is_match(b"ac"));
        assert!(regex.is_match(b"abc"));
        assert!(regex.is_match(b"abbbc"));
    }

    #[test]
    fn regex_lazy_one_or_more() {
        let regex = Regex::new(r"ab+?").unwrap();
        assert!(regex.is_match(b"ab"));
        let cap = regex.captures(&mut b"abbbb").next().expect("Did not match");
        assert_eq!(cap_len(&cap[0]), 2);
        assert!(!regex.is_match(b"xaxbxx"));
        assert!(!regex.is_match(b"a"));
        assert!(!regex.is_match(b"b"));
    }

    #[test]
    fn regex_lazy_counted_rep_capped() {
        let regex = Regex::new(r"\d{3,5}?").unwrap();
        assert!(regex.is_match(b"222"));
        let cap = regex
            .captures(&mut b"1111111111111")
            .next()
            .expect("Did not match");
        assert_eq!(cap_len(&cap[0]), 3);

        assert!(!regex.is_match(b"13"));
        assert!(!regex.is_match(b"0"));
        assert!(!regex.is_match(b"9"));
    }

    #[test]
    fn regex_lazy_counted_rep_inf() {
        let regex = Regex::new(r"\d{3,}?").unwrap();
        assert!(regex.is_match(b"222"));
        let cap = regex
            .captures(&mut b"1111111111111")
            .next()
            .expect("Did not match");
        assert_eq!(cap_len(&cap[0]), 3);

        assert!(!regex.is_match(b"13"));
        assert!(!regex.is_match(b"0"));
        assert!(!regex.is_match(b"9"));

        let regex = Regex::new(r"\d{3,}?a").unwrap();
        assert!(regex.is_match(b"123456789a"));
        assert!(!regex.is_match(b"123456789b"));
    }

    #[test]
    fn regex_utf8_range() {
        let regex = Regex::new(r"[Ά-Ϋ]+").unwrap();
        assert!(regex.is_match(&"ΨΕΖ".as_bytes()));
        assert!(!regex.is_match(b"a"));
        assert!(!regex.is_match(b"A"));
        assert!(!regex.is_match(b"Y"));
        assert!(!regex.is_match(b"0"));
    }
}
