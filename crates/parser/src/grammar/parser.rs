mod rule;

use std::io;
use std::mem;

use anyhow::bail;
use anyhow::Result;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use sanedit_utils::ranges::OverlappingRanges;

pub use self::rule::Annotation;
pub(crate) use self::rule::Rule;
pub(crate) use self::rule::RuleInfo;
pub(crate) use self::rule::Rules;

use super::lexer::Lexer;
use super::lexer::Token;

pub(crate) fn parse_rules_from_str(input: &str) -> Result<Rules> {
    let sinput = io::Cursor::new(input);
    parse_rules(sinput)
}

pub(crate) fn parse_rules<R: io::Read>(read: R) -> Result<Rules> {
    let mut lex = Lexer::new(read);
    let token = lex.next()?;
    let parser = GrammarParser {
        lex,
        token,
        rules: vec![],
        indices: FxHashMap::default(),
        seen: FxHashSet::default(),
        cp: 0,
    };
    parser.parse()
}

pub(crate) const CHAR_MIN: char = '\u{0}';
pub(crate) const CHAR_MAX: char = '\u{10ffff}';

// Operator      Priority
// (e)           5
// e*, e+, e?    4
// &e, !e        3
// e1 e2         2
// e1 / e2       1

#[derive(Debug)]
pub(crate) struct GrammarParser<R: io::Read> {
    lex: Lexer<R>,
    token: Token,

    /// Seen rules, used to identify rules that are referenced but not defined
    seen: FxHashSet<String>,
    /// All the parsed rules
    rules: Vec<RuleInfo>,
    /// Map from rule name to its index
    indices: FxHashMap<String, usize>,

    cp: usize,
}

impl<R: io::Read> GrammarParser<R> {
    fn parse(mut self) -> Result<Rules> {
        while self.token != Token::EOF {
            let rule = self.rule()?;
            self.seen.insert(rule.name.clone());

            match self.indices.get(&rule.name) {
                Some(i) => {
                    self.rules[*i] = rule;
                }
                None => {
                    let i = self.rules.len();
                    self.indices.insert(rule.name.clone(), i);
                    self.rules.push(rule);
                }
            }
        }

        self.apply_annotations()?;
        self.validate()?;

        let rules = self.rules.into();
        Ok(Rules::new(rules))
    }

    fn validate(&mut self) -> Result<()> {
        for rule in &self.rules {
            if !self.seen.contains(&rule.name) {
                bail!("Referenced non existent rule: {}", rule.name);
            }
        }

        Ok(())
    }

    fn apply_annotations(&mut self) -> Result<()> {
        const WHITESPACE_RULE: &str = "WHITESPACE";
        let ws_ann = self
            .rules
            .iter()
            .any(|r| r.annotations.contains(&Annotation::Whitespaced));

        // Apply whitespaced
        if ws_ann {
            use Rule::*;
            let i = match self.indices.get(WHITESPACE_RULE) {
                Some(i) => *i,
                None => bail!("WHITESPACE rule required when using annotation @whitespaced"),
            };
            let ws_rule = &self.rules[i];
            let ws_def = ws_rule.rule.clone().into();
            let ws_zom = ZeroOrMore(ws_def);

            let len = self.rules.len();
            for rule in &mut self.rules {
                if rule.annotations.contains(&Annotation::Whitespaced) {
                    rule.apply_whitespaced(len);
                }
            }

            const WS_NAME: &str = "WS*";
            self.rules.push(RuleInfo {
                name: WS_NAME.into(),
                rule: ws_zom,
                annotations: vec![],
                top: false,
            });
            self.seen.insert(WS_NAME.into());
        }

        Ok(())
    }

    fn annotation(&mut self) -> Result<Option<(String, Option<String>)>> {
        if self.token == Token::Annotation {
            self.consume(Token::Annotation)?;
            let ann = self.text()?;

            let mut specifiers = None;
            if self.token == Token::LParen {
                self.consume(Token::LParen)?;
                specifiers = Some(self.text()?);
                self.consume(Token::RParen)?;
            }
            Ok(Some((ann, specifiers)))
        } else {
            Ok(None)
        }
    }

    fn annotations(&mut self) -> Result<Vec<Annotation>> {
        let mut anns = vec![];
        while let Some((ann, specifiers)) = self.annotation()? {
            match ann.as_str() {
                "whitespaced" => anns.push(Annotation::Whitespaced),
                "show" => anns.push(Annotation::Show(specifiers)),
                _ => anns.push(Annotation::Other(ann, specifiers)),
            }
        }

        Ok(anns)
    }

    fn rule(&mut self) -> Result<RuleInfo> {
        let top = self.rules.is_empty();
        let annotations = self.annotations()?;
        let name = self.text()?;
        self.consume(Token::Assign)?;
        let def = self.rule_def()?;
        self.consume(Token::End)?;

        let rule = RuleInfo {
            top,
            name,
            annotations,
            rule: def,
        };
        Ok(rule)
    }

    fn rule_def(&mut self) -> Result<Rule> {
        self.choice()
    }

    fn choice(&mut self) -> Result<Rule> {
        let mut rules = vec![];

        loop {
            let start = self.lex.token_count();
            match self.sequence() {
                Ok(r) => rules.push(r),
                Err(e) => {
                    let end = self.lex.token_count();
                    if start == end {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }

            if let Err(_) = self.consume(Token::Choice) {
                break;
            }
        }

        if rules.is_empty() {
            bail!("Failed to create choice at: {}", self.lex.pos())
        }

        if rules.len() > 1 {
            Ok(Rule::Choice(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn sequence(&mut self) -> Result<Rule> {
        let mut rules = vec![];

        loop {
            let start = self.lex.token_count();
            match self.simple_rule() {
                Ok(r) => rules.push(r),
                Err(e) => {
                    let end = self.lex.token_count();
                    if start == end {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if rules.is_empty() {
            bail!("Failed to create sequence at: {}", self.lex.pos())
        }

        if rules.len() > 1 {
            Ok(Rule::Sequence(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn simple_rule(&mut self) -> Result<Rule> {
        // Prefix + rule
        let mut rule = match &self.token {
            Token::And => {
                self.consume(Token::And)?;
                let rule = self.simple_rule()?;
                Rule::FollowedBy(rule.into())
            }
            Token::Not => {
                self.consume(Token::Not)?;
                let rule = self.simple_rule()?;
                Rule::NotFollowedBy(rule.into())
            }
            Token::LParen => {
                self.consume(Token::LParen)?;
                let rule = self.rule_def()?;
                self.consume(Token::RParen)?;
                rule
            }
            Token::LBracket => {
                self.consume(Token::LBracket)?;
                let rule = self.brackets()?;
                self.consume(Token::RBracket)?;
                rule
            }
            Token::Quote => {
                self.consume(Token::Quote)?;
                let literal = self.text()?;
                self.consume(Token::Quote)?;
                Rule::ByteSequence(literal.into())
            }
            Token::Any => {
                self.consume(Token::Any)?;
                Rule::ByteAny
            }
            Token::Text(_) => {
                let ref_rule = self.text()?;

                match self.indices.get(&ref_rule) {
                    Some(i) => Rule::Ref(*i),
                    None => {
                        let i = self.rules.len();
                        let rrule = RuleInfo {
                            name: ref_rule.clone(),
                            rule: Rule::Ref(0),
                            annotations: vec![],
                            top: false,
                        };
                        self.indices.insert(ref_rule, i);
                        self.rules.push(rrule);
                        Rule::Ref(i)
                    }
                }
            }
            _ => bail!("Unexpected token {:?} while parsing rule", self.token),
        };

        // postfix
        match self.token {
            Token::ZeroOrMore => {
                self.consume(Token::ZeroOrMore)?;
                rule = Rule::ZeroOrMore(rule.into());
            }
            Token::OneOrMore => {
                self.consume(Token::OneOrMore)?;
                rule = Rule::OneOrMore(rule.into())
            }
            Token::Optional => {
                self.consume(Token::Optional)?;
                rule = Rule::Optional(rule.into());
            }
            _ => {}
        }

        Ok(rule)
    }

    /// Consumes the provided token if it matches current token and advances to
    /// the next token
    fn consume(&mut self, tok: Token) -> Result<()> {
        if tok == self.token {
            self.token = self.lex.next()?;
            Ok(())
        } else {
            bail!(
                "Expected token {tok:?} but got {:?}, at {}",
                self.token,
                self.lex.pos()
            )
        }
    }

    /// Returns the current token and advances to the next
    fn skip(&mut self) -> Result<Token> {
        let token = mem::replace(&mut self.token, self.lex.next()?);
        Ok(token)
    }

    fn text(&mut self) -> Result<String> {
        let pos = self.lex.pos();
        match self.skip()? {
            Token::Text(s) => Ok(s),
            tok => {
                bail!("Expected a string but got {:?}, at {pos}", tok,)
            }
        }
    }

    fn char(&mut self) -> Result<char> {
        let pos = self.lex.pos();
        match self.skip()? {
            Token::Char(c) => Ok(c),
            tok => {
                bail!("Expected a character but got {:?}, at {pos}", tok,)
            }
        }
    }

    fn brackets(&mut self) -> Result<Rule> {
        let mut choices = vec![];
        let mut range = false;
        let negate = self.token == Token::Negate;
        if negate {
            self.skip()?;
        }

        while self.token != Token::RBracket {
            match &self.token {
                Token::Byte(b) => {
                    if range {
                        range = false;
                        let start = choices
                            .last()
                            .map(|rule| match rule {
                                Rule::ByteSequence(bytes) => {
                                    if bytes.len() == 1 {
                                        Some(bytes[0])
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            })
                            .flatten();
                        match start {
                            Some(a) => {
                                choices.pop();
                                choices.push(Rule::ByteRange(a, *b));
                            }
                            None => bail!(
                                "Tried to create range with invalid byte at: {}",
                                self.lex.pos()
                            ),
                        }
                    } else {
                        choices.push(Rule::ByteSequence(vec![*b]));
                    }

                    self.skip()?;
                }
                Token::Range => {
                    if range {
                        bail!("Found another range separator at {}", self.lex.pos());
                    }
                    self.consume(Token::Range)?;
                    range = true;
                }
                Token::Char(ch) => {
                    if range {
                        range = false;
                        let start = choices
                            .last()
                            .map(|rule| match rule {
                                Rule::ByteSequence(seq) => std::str::from_utf8(seq)
                                    .ok()
                                    .map(|s| s.chars().next())
                                    .flatten(),
                                _ => None,
                            })
                            .flatten();
                        match start {
                            Some(a) => {
                                choices.pop();
                                choices.push(Rule::UTF8Range(a, *ch));
                            }
                            None => bail!(
                                "Tried to create range with invalid character at: {}",
                                self.lex.pos()
                            ),
                        }
                    } else {
                        let mut buf = [0; 4];
                        let utf8_ch = ch.encode_utf8(&mut buf);
                        choices.push(Rule::ByteSequence(utf8_ch.as_bytes().into()));
                    }

                    self.skip()?;
                }
                _ => bail!(
                    "Encountered {:?} while in brackets at {}",
                    self.token,
                    self.lex.pos()
                ),
            }
        }

        if negate {
            // If we have a single unicode element in choices assume unicode
            let mut utf8 = false;
            let mut ranges = OverlappingRanges::default();
            for choice in &choices {
                match choice {
                    Rule::ByteSequence(bytes) => {
                        // Can be a char or byte
                        utf8 |= bytes.len() != 1;

                        if utf8 {
                            match std::str::from_utf8(bytes)
                                .map(|s| s.chars().next())
                                .ok()
                                .flatten()
                            {
                                Some(ch) => {
                                    ranges.add(ch as usize..ch as usize + 1);
                                }
                                None => bail!(
                                    "Failed to convert byte sequence {:?} to utf8 character at {}",
                                    bytes,
                                    self.lex.pos()
                                ),
                            }
                        } else {
                            let byte = bytes[0] as usize;
                            ranges.add(byte..byte + 1);
                        }
                    }
                    Rule::ByteRange(a, b) => {
                        ranges.add(*a as usize..*b as usize + 1);
                    }
                    Rule::UTF8Range(a, b) => {
                        utf8 = true;
                        ranges.add(*a as usize..*b as usize + 1);
                    }
                    _ => unreachable!(),
                }
            }

            if utf8 {
                ranges.invert(CHAR_MIN as usize..CHAR_MAX as usize + 1);
                choices.clear();
                for range in ranges.iter() {
                    let start = char::from_u32(range.start as u32);
                    let end = char::from_u32(range.end as u32 - 1);

                    match (start, end) {
                        (Some(a), Some(b)) => choices.push(Rule::UTF8Range(a, b)),
                        _ => bail!("Failed to convert ranges back to char ranges"),
                    }
                }
            } else {
                ranges.invert(u8::MIN as usize..u8::MAX as usize + 1);
                choices.clear();
                for range in ranges.iter() {
                    choices.push(Rule::ByteRange(range.start as u8, (range.end - 1) as u8));
                }
            }
        }

        Ok(Rule::Choice(choices))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn print_rules(rules: &[RuleInfo]) -> String {
        let mut result = String::new();

        for (i, rule) in rules.iter().enumerate() {
            let srule = format!("{i}: {}: {};\n", &rule.name, &rule.rule);
            result.push_str(&srule);
        }
        result
    }

    #[test]
    fn grammar_json() {
        let peg = include_str!("../../pegs/json.peg");
        match parse_rules_from_str(peg) {
            Ok(rules) => println!("==== Created rules ====\n{}", print_rules(&rules)),
            Err(e) => println!("Failed to create rules: {e}"),
        }
    }
}
