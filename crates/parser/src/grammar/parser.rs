use std::collections::HashMap;
use std::mem;

use crate::input::Input;
use crate::input::StringInput;
use anyhow::bail;
use anyhow::Result;

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
        clauses: vec![],
        indices: HashMap::new(),
    };
    parser.parse()
}

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    pub(crate) name: String,
    pub(crate) clause: Clause,
}

#[derive(Debug, Clone)]
pub(crate) enum Clause {
    OneOrMore(Box<Clause>),
    Choice(Vec<Clause>),
    Sequence(Vec<Clause>),
    FollowedBy(Box<Clause>),
    NotFollowedBy(Box<Clause>),
    CharSequence(String),
    Ref(usize),
    Nothing,
}

impl Clause {
    fn is_terminal(&self) -> bool {
        use Clause::*;
        match self {
            Nothing | CharSequence(_) => true,
            _ => false,
        }
    }

    fn is_nothing(&self) -> bool {
        matches!(self, Clause::Nothing)
    }
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

    clauses: Vec<Rule>,
    indices: HashMap<String, usize>,
}

impl<I: Input> GrammarParser<I> {
    fn parse(mut self) -> Result<Box<[Rule]>> {
        while self.token != Token::EOF {
            let rule = self.rule()?;

            match self.indices.get(&rule.name) {
                Some(i) => {
                    self.clauses[*i] = rule;
                }
                None => {
                    let i = self.clauses.len();
                    self.indices.insert(rule.name.clone(), i);
                    self.clauses.push(rule);
                }
            }
        }

        Ok(self.clauses.into())
    }

    fn rule(&mut self) -> Result<Rule> {
        let name = self.text()?;
        self.consume(Token::Assign)?;
        let clause = self.clauses()?;
        self.consume(Token::End)?;
        Ok(Rule { name, clause })
    }

    fn clauses(&mut self) -> Result<Clause> {
        self.choice()
    }

    fn choice(&mut self) -> Result<Clause> {
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
            Ok(Clause::Choice(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn sequence(&mut self) -> Result<Clause> {
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
            Ok(Clause::Sequence(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn clause(&mut self) -> Result<Clause> {
        // Prefix + rule
        let mut rule = match &self.token {
            Token::And => {
                self.consume(Token::And)?;
                let rule = self.clauses()?;
                Clause::FollowedBy(rule.into())
            }
            Token::Not => {
                self.consume(Token::Not)?;
                let rule = self.clauses()?;
                Clause::NotFollowedBy(rule.into())
            }
            Token::LParen => {
                self.consume(Token::LParen)?;
                let rule = self.clauses()?;
                self.consume(Token::RParen)?;
                rule
            }
            // Token::LBracket => {
            //     self.consume(Token::LParen)?;
            //     let rule = self.rule()?;
            //     self.consume(Token::RParen)?;
            // }
            Token::Quote => {
                self.consume(Token::Quote)?;
                let literal = self.text()?;
                self.consume(Token::Quote)?;
                Clause::CharSequence(literal)
            }
            Token::Text(_) => {
                let ref_rule = self.text()?;
                match self.indices.get(&ref_rule) {
                    Some(i) => Clause::Ref(*i),
                    None => {
                        let i = self.clauses.len();
                        let rrule = Rule {
                            name: ref_rule,
                            clause: Clause::Nothing,
                        };
                        self.indices.insert(rrule.name.clone(), i);
                        self.clauses.push(rrule);
                        Clause::Ref(i)
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
                choices.push(Clause::OneOrMore(rule.into()));
                choices.push(Clause::Nothing);
                rule = Clause::Choice(choices)
            }
            Token::OneOrMore => {
                self.consume(Token::OneOrMore)?;
                rule = Clause::OneOrMore(rule.into())
            }
            Token::Optional => {
                self.consume(Token::Optional)?;
                let mut choices = Vec::with_capacity(2);
                choices.push(rule);
                choices.push(Clause::Nothing);
                rule = Clause::Choice(choices)
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
                bail!("Expected an identifier but got {:?}, at {pos}", tok,)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn print_rules(rules: &[Rule]) -> String {
        let mut result = String::new();

        for (i, rule) in rules.iter().enumerate() {
            let srule = format!("{i}: {}: {};\n", &rule.name, print_rule(&rule.clause));
            result.push_str(&srule);
        }
        result
    }

    fn print_rule(rule: &Clause) -> String {
        match rule {
            Clause::CharSequence(l) => format!("\"{}\"", l),
            Clause::Choice(choices) => {
                let mut result = String::new();
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" | ");
                    }

                    result.push_str(&print_rule(choice));
                }

                result
            }
            Clause::Sequence(seq) => {
                let mut result = String::new();
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&print_rule(choice));
                }

                result
            }
            Clause::NotFollowedBy(r) => format!("!({})", print_rule(r)),
            Clause::FollowedBy(r) => format!("&({})", print_rule(r)),
            Clause::Ref(r) => format!("r\"{r}\""),
            Clause::OneOrMore(r) => format!("({})*", print_rule(r)),
            Clause::Nothing => format!("()"),
        }
    }

    #[test]
    fn calc() {
        let peg = include_str!("../../pegs/calc.peg");
        match parse_rules_from_str(peg) {
            Ok(rules) => println!("==== Created rules ====\n{}", print_rules(&rules)),
            Err(e) => println!("Failed to create rules: {e}"),
        }
    }
}
