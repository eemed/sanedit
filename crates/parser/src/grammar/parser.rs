use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

use crate::input::Input;
use anyhow::bail;
use anyhow::Result;

use super::lexer::Lexer;
use super::lexer::Token;

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    name: String,
    clause: Rc<Clause>,
}

#[derive(Debug, Clone)]
pub(crate) enum Clause {
    OneOrMore(Box<Clause>),
    Choice(Vec<Clause>),
    Sequence(Vec<Clause>),
    FollowedBy(Box<Clause>),
    NotFollowedBy(Box<Clause>),
    CharSequence(String),
    Ref(String),
    Nothing,
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
}

impl<I: Input> GrammarParser<I> {
    fn parse(input: I) -> Result<Vec<Clause>> {
        let mut lex = Lexer::new(input);
        let token = lex.next()?;
        let mut parser = GrammarParser { lex, token };

        let mut map = HashMap::new();
        while parser.token != Token::EOF {
            let (name, clause) = parser.rule()?;
            map.insert(name, clause);
        }

        todo!()
        // parser.parsed.sort_by(|a, b| a.name.cmp(&b.name));
        // let rules = mem::take(&mut parser.parsed);
        // Ok(rules)
    }

    fn rule(&mut self) -> Result<(String, Clause)> {
        let name = self.text()?;
        self.consume(Token::Assign)?;
        let clause = self.clauses()?;
        self.consume(Token::End)?;
        Ok((name, clause))
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
                Clause::Ref(ref_rule)
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

    fn print_rules(map: HashMap<String, Clause>) -> String {
        let mut result = String::new();
        let mut rules: Vec<(String, Clause)> = map.into_iter().collect();
        rules.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (name, rule) in rules {
            let srule = format!("{}: {};\n", name, print_rule(&rule));
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
            Clause::Ref(r) => r.clone(),
            Clause::OneOrMore(r) => format!("({})*", print_rule(r)),
            Clause::Nothing => format!("()"),
        }
    }

    #[test]
    fn calc() {
        let peg = include_str!("../../pegs/calc.peg");
        match parse_rules_from_str(peg) {
            Ok(rules) => println!("==== Created rules ====\n{}", print_rules(rules)),
            Err(e) => println!("Failed to create rules: {e}"),
        }
    }
}
