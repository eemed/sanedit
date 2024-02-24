use std::collections::HashMap;
use std::mem;

use crate::input::Input;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Result;

use super::lexer::Lexer;
use super::lexer::Token;

#[derive(Debug)]
enum Rule {
    Literal(String),
    Repetion(u32, u32, Box<Rule>),
    Choice(Vec<Rule>),
    Sequence(Vec<Rule>),
    Not(Box<Rule>),
    And(Box<Rule>),
    Ref(String),
}

struct Parser<I: Input> {
    lex: Lexer<I>,
    token: Token,
}

impl<I: Input> Parser<I> {
    fn parse(input: I) -> Result<HashMap<String, Rule>> {
        let mut lex = Lexer::new(input);
        let token = lex.next()?;
        let mut parser = Parser { lex, token };
        let mut rules = HashMap::new();

        while parser.token != Token::EOF {
            let (name, rule) = parser.named_rule()?;
            rules.insert(name, rule);
        }

        Ok(rules)
    }

    fn named_rule(&mut self) -> Result<(String, Rule)> {
        let name = self.text()?;
        self.consume(Token::Assign)?;
        let rule = self.rule()?;
        println!("Parsed: {name}: {rule:?}");
        self.consume(Token::End)?;
        Ok((name, rule))
    }

    fn rule(&mut self) -> Result<Rule> {
        let rule = self.rule_single()?;

        match self.token {
            Token::ZeroOrMore => {
                self.consume(Token::ZeroOrMore)?;
                Ok(Rule::Repetion(0, u32::MAX, Box::new(rule)))
            }
            Token::OneOrMore => {
                self.consume(Token::OneOrMore)?;
                Ok(Rule::Repetion(1, u32::MAX, Box::new(rule)))
            }
            Token::Optional => {
                self.consume(Token::Optional)?;
                Ok(Rule::Repetion(0, 1, Box::new(rule)))
            }
            Token::Choice => self.choice(rule),
            _ => self.sequence(rule),
        }
    }

    fn sequence(&mut self, rule: Rule) -> Result<Rule> {
        let mut rules = match rule {
            Rule::Sequence(rules) => rules,
            _ => vec![rule],
        };

        loop {
            match self.rule() {
                Ok(r) => rules.push(r),
                Err(e) => {
                    break;
                }
            }
        }

        if rules.len() > 1 {
            Ok(Rule::Sequence(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
    }

    fn choice(&mut self, rule: Rule) -> Result<Rule> {
        let mut rules = match rule {
            Rule::Choice(rules) => rules,
            _ => vec![rule],
        };

        loop {
            if let Err(_) = self.consume(Token::Choice) {
                break;
            }

            match self.rule() {
                Ok(r) => rules.push(r),
                Err(e) => {
                    break;
                }
            }
        }

        Ok(Rule::Choice(rules))
    }

    fn rule_single(&mut self) -> Result<Rule> {
        match &self.token {
            Token::And => todo!(),
            Token::Not => {
                self.consume(Token::Not)?;
                let rule = self.rule()?;
                Ok(Rule::Not(Box::new(rule)))
            }
            Token::LParen => {
                self.consume(Token::LParen)?;
                let rule = self.rule()?;
                self.consume(Token::RParen)?;
                Ok(rule)
            }
            Token::Quote => {
                self.consume(Token::Quote)?;
                let literal = self.text()?;
                self.consume(Token::Quote)?;
                Ok(Rule::Literal(literal))
            }
            Token::Text(_) => {
                let text = self.text()?;
                Ok(Rule::Ref(text))
            }
            _ => bail!("Unexpected token {:?} while parsing rule", self.token),
        }
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
        match self.skip()? {
            Token::Text(s) => Ok(s),
            tok => {
                bail!(
                    "Expected an identifier but got {:?}, at {}",
                    tok,
                    self.lex.pos()
                )
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::input::StringInput;

    #[test]
    fn calc() {
        let peg = include_str!("../../pegs/calc.peg");
        let input = StringInput::new(peg);
        let res = Parser::parse(input);
        println!("res: {res:?}");
    }
}
