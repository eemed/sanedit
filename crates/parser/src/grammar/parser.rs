mod rule;

use std::collections::HashMap;
use std::mem;

use crate::input::Input;
use crate::input::StringInput;
use anyhow::bail;
use anyhow::Result;

pub(crate) use self::rule::Annotation;
pub(crate) use self::rule::Rule;
pub(crate) use self::rule::RuleDefinition;

use super::lexer::Lexer;
use super::lexer::Token;

pub(crate) fn parse_rules_from_str(input: &str) -> Result<Box<[Rule]>> {
    let sinput = StringInput::new(input);
    parse_rules(sinput)
}

pub(crate) fn parse_rules<I: Input>(input: I) -> Result<Box<[Rule]>> {
    let mut lex = Lexer::new(input);
    let token = lex.next()?;
    let mut parser = GrammarParser {
        lex,
        token,
        rules: vec![],
        indices: HashMap::new(),
    };
    parser.parse()
}

// Operator      Priority
// (e)           5
// e*, e+, e?    4
// &e, !e        3
// e1 e2         2
// e1 / e2       1

#[derive(Debug)]
pub(crate) struct GrammarParser<I: Input> {
    lex: Lexer<I>,
    token: Token,

    rules: Vec<Rule>,
    indices: HashMap<String, usize>,
}

impl<I: Input> GrammarParser<I> {
    fn parse(mut self) -> Result<Box<[Rule]>> {
        while self.token != Token::EOF {
            let rule = self.rule()?;

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

        Ok(self.rules.into())
    }

    fn apply_annotations(&mut self) -> Result<()> {
        const WHITESPACE_RULE: &str = "WHITESPACE";
        let ws_ann = self
            .rules
            .iter()
            .any(|r| r.annotations.contains(&Annotation::Whitespaced));

        // Apply whitespaced
        if ws_ann {
            use RuleDefinition::*;
            let i = match self.indices.get(WHITESPACE_RULE) {
                Some(i) => *i,
                None => bail!("WHITESPACE rule required when using annotation @whitespaced"),
            };
            let ws_rule = &self.rules[i];
            let ws_def = ws_rule.def.clone().into();
            let ws_zom = Choice(vec![OneOrMore(ws_def), Nothing]);

            let len = self.rules.len();
            for rule in &mut self.rules {
                if rule.annotations.contains(&Annotation::Whitespaced) {
                    rule.apply_whitespaced(len);
                }
            }

            self.rules.push(Rule {
                name: "WS*".into(),
                def: ws_zom,
                annotations: vec![],
            });
        }

        Ok(())
    }

    fn annotation(&mut self) -> Result<Option<String>> {
        if self.token == Token::Annotation {
            self.consume(Token::Annotation)?;
            let text = self.text()?;
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    fn annotations(&mut self) -> Result<Vec<Annotation>> {
        let mut anns = vec![];
        while let Some(ann) = self.annotation()? {
            match ann.as_str() {
                "whitespaced" => anns.push(Annotation::Whitespaced),
                "show" => anns.push(Annotation::Show),
                a => bail!("Unexpected annotation {a}"),
            }
        }

        Ok(anns)
    }

    fn rule(&mut self) -> Result<Rule> {
        let annotations = self.annotations()?;
        let name = self.text()?;
        self.consume(Token::Assign)?;
        let def = self.rule_def()?;
        self.consume(Token::End)?;
        let rule = Rule {
            name,
            annotations,
            def,
        };
        Ok(rule)
    }

    fn rule_def(&mut self) -> Result<RuleDefinition> {
        self.choice()
    }

    fn choice(&mut self) -> Result<RuleDefinition> {
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

        if rules.len() > 1 {
            Ok(RuleDefinition::Choice(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn sequence(&mut self) -> Result<RuleDefinition> {
        let mut rules = vec![];

        loop {
            let start = self.lex.token_count();
            match self.clause() {
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

        if rules.len() > 1 {
            Ok(RuleDefinition::Sequence(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn clause(&mut self) -> Result<RuleDefinition> {
        // Prefix + rule
        let mut rule = match &self.token {
            Token::And => {
                self.consume(Token::And)?;
                let rule = self.rule_def()?;
                RuleDefinition::FollowedBy(rule.into())
            }
            Token::Not => {
                self.consume(Token::Not)?;
                let rule = self.rule_def()?;
                RuleDefinition::NotFollowedBy(rule.into())
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
                RuleDefinition::CharSequence(literal)
            }
            Token::Text(_) => {
                let ref_rule = self.text()?;

                match self.indices.get(&ref_rule) {
                    Some(i) => RuleDefinition::Ref(*i),
                    None => {
                        let i = self.rules.len();
                        let rrule = Rule {
                            name: ref_rule.clone(),
                            def: RuleDefinition::Nothing,
                            annotations: vec![],
                        };
                        self.indices.insert(ref_rule, i);
                        self.rules.push(rrule);
                        RuleDefinition::Ref(i)
                    }
                }
            }
            _ => bail!("Unexpected token {:?} while parsing rule", self.token),
        };

        // postfix
        match self.token {
            Token::ZeroOrMore => {
                self.consume(Token::ZeroOrMore)?;
                let mut choices = Vec::with_capacity(2);
                choices.push(RuleDefinition::OneOrMore(rule.into()));
                choices.push(RuleDefinition::Nothing);
                rule = RuleDefinition::Choice(choices)
            }
            Token::OneOrMore => {
                self.consume(Token::OneOrMore)?;
                rule = RuleDefinition::OneOrMore(rule.into())
            }
            Token::Optional => {
                self.consume(Token::Optional)?;
                let mut choices = Vec::with_capacity(2);
                choices.push(rule);
                choices.push(RuleDefinition::Nothing);
                rule = RuleDefinition::Choice(choices)
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

    fn brackets(&mut self) -> Result<RuleDefinition> {
        let mut choices = vec![];
        let mut range = false;
        while self.token != Token::RBracket {
            println!("Token: {:?}", self.token);
            match &self.token {
                Token::Char(ch) => {
                    if range {
                        range = false;
                        let a = choices
                            .last()
                            .map(|r| match r {
                                RuleDefinition::CharSequence(c) => {
                                    let mut iter = c.chars();
                                    let c = iter.next()?;
                                    if iter.next().is_some() {
                                        None
                                    } else {
                                        Some(c)
                                    }
                                }
                                _ => None,
                            })
                            .flatten();
                        match a {
                            Some(a) => {
                                choices.pop();
                                choices.push(RuleDefinition::CharRange(a, *ch));
                            }
                            None => bail!(
                                "Tried to create range with invalid character at: {}",
                                self.lex.pos()
                            ),
                        }
                    } else {
                        choices.push(RuleDefinition::CharSequence(ch.to_string()));
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
                _ => bail!(
                    "Encountered {:?} while in brackets at {}",
                    self.token,
                    self.lex.pos()
                ),
            }
        }

        Ok(RuleDefinition::Choice(choices))
    }

    fn char_range(&mut self) -> Result<RuleDefinition> {
        let a = self.char()?;
        self.consume(Token::Range)?;
        let b = self.char()?;
        Ok(RuleDefinition::CharRange(a, b))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn print_rules(rules: &[Rule]) -> String {
        let mut result = String::new();

        for (i, rule) in rules.iter().enumerate() {
            let srule = format!("{i}: {}: {};\n", &rule.name, print_rule(&rule.def));
            result.push_str(&srule);
        }
        result
    }

    fn print_rule(rule: &RuleDefinition) -> String {
        match rule {
            RuleDefinition::CharSequence(l) => format!("\"{}\"", l),
            RuleDefinition::Choice(choices) => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" / ");
                    }

                    result.push_str(&print_rule(choice));
                }
                result.push_str(")");

                result
            }
            RuleDefinition::Sequence(seq) => {
                let mut result = String::new();
                result.push_str("(");
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&print_rule(choice));
                }
                result.push_str(")");

                result
            }
            RuleDefinition::NotFollowedBy(r) => format!("!({})", print_rule(r)),
            RuleDefinition::FollowedBy(r) => format!("&({})", print_rule(r)),
            RuleDefinition::Ref(r) => format!("r\"{r}\""),
            RuleDefinition::OneOrMore(r) => format!("({})*", print_rule(r)),
            RuleDefinition::Nothing => format!("()"),
            RuleDefinition::CharRange(_, _) => todo!(),
        }
    }

    #[test]
    fn grammar_calc() {
        let peg = include_str!("../../pegs/calc.peg");
        match parse_rules_from_str(peg) {
            Ok(rules) => println!("==== Created rules ====\n{}", print_rules(&rules)),
            Err(e) => println!("Failed to create rules: {e}"),
        }
    }
}
