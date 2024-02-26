use std::collections::HashMap;
use std::mem;

use crate::input::Input;
use crate::input::StringInput;
use anyhow::bail;
use anyhow::Result;

use super::lexer::Lexer;
use super::lexer::Token;

#[derive(Debug, Clone)]
pub(crate) enum Clause {
    Literal(String),
    Repetion(u32, u32, Box<Clause>),
    Choice(Vec<Clause>),
    Sequence(Vec<Clause>),
    Not(Box<Clause>),
    And(Box<Clause>),
    Ref(String),
}

impl Clause {
    pub fn is_terminal(&self) -> bool {
        match self {
            Clause::Literal(_) => true,
            _ => false,
        }
    }
}

// Operator      Priority
// (e)           5
// e*, e+, e?    4
// &e, !e        3
// e1 e2         2
// e1 / e2       1

pub(crate) fn parse_from_str(grammar: &str) -> Result<HashMap<String, Clause>> {
    let input = StringInput::new(grammar);
    parse(input)
}

pub(crate) fn parse<I: Input>(input: I) -> Result<HashMap<String, Clause>> {
    let mut lex = Lexer::new(input);
    let token = lex.next()?;
    let mut parser = GrammarParser { lex, token };
    let mut rules = HashMap::new();

    while parser.token != Token::EOF {
        let (name, rule) = parser.rule()?;
        rules.insert(name, rule);
    }

    // TODO optimize referencese out of rules
    Ok(rules)
}

#[derive(Debug)]
pub(crate) struct GrammarParser<I: Input> {
    lex: Lexer<I>,
    token: Token,
}

impl<I: Input> GrammarParser<I> {
    fn rule(&mut self) -> Result<(String, Clause)> {
        let name = self.text()?;
        self.consume(Token::Assign)?;
        let rule = self.clauses()?;
        self.consume(Token::End)?;
        Ok((name, rule))
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
                Clause::And(Box::new(rule))
            }
            Token::Not => {
                self.consume(Token::Not)?;
                let rule = self.clauses()?;
                Clause::Not(Box::new(rule))
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
                Clause::Literal(literal)
            }
            Token::Text(_) => {
                let text = self.text()?;
                Clause::Ref(text)
            }
            _ => bail!("Unexpected token {:?} while parsing rule", self.token),
        };

        // postfix
        match self.token {
            Token::ZeroOrMore => {
                self.consume(Token::ZeroOrMore)?;
                rule = Clause::Repetion(0, u32::MAX, Box::new(rule))
            }
            Token::OneOrMore => {
                self.consume(Token::OneOrMore)?;
                rule = Clause::Repetion(1, u32::MAX, Box::new(rule))
            }
            Token::Optional => {
                self.consume(Token::Optional)?;
                rule = Clause::Repetion(0, 1, Box::new(rule));
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
            Clause::Literal(l) => format!("\"{}\"", l),
            Clause::Repetion(n, m, r) => {
                let mark = match (n, m) {
                    (0, 1) => "?",
                    (0, _) => "*",
                    (1, _) => "+",
                    _ => unreachable!("no such repetition"),
                };
                format!("({}){}", print_rule(r), mark)
            }
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
            Clause::Not(r) => format!("!({})", print_rule(r)),
            Clause::And(r) => format!("&({})", print_rule(r)),
            Clause::Ref(r) => r.clone(),
        }
    }

    #[test]
    fn calc() {
        let peg = include_str!("../../pegs/calc.peg");
        match parse_from_str(peg) {
            Ok(rules) => println!("==== Created rules ====\n{}", print_rules(rules)),
            Err(e) => println!("Failed to create rules: {e}"),
        }
    }
}
