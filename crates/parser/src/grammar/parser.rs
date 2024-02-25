use std::collections::HashMap;
use std::mem;

use crate::input::Input;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Result;

use super::lexer::Lexer;
use super::lexer::Token;

#[derive(Debug, Clone)]
enum Rule {
    Literal(String),
    Repetion(u32, u32, Box<Rule>),
    Choice(Vec<Rule>),
    Sequence(Vec<Rule>),
    Not(Box<Rule>),
    And(Box<Rule>),
    Ref(String),
}

// Operator      Priority
// (e)           5
// e*, e+, e?    4
// &e, !e        3
// e1 e2         2
// e1 / e2       1

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
        self.consume(Token::End)?;
        Ok((name, rule))
    }

    fn rule(&mut self) -> Result<Rule> {
        self.choice()
    }

    fn sequence(&mut self) -> Result<Rule> {
        let mut rules = vec![];

        loop {
            let start = self.lex.token_count();
            match self.rule_single() {
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
            Ok(Rule::Sequence(rules))
        } else {
            Ok(rules.pop().unwrap())
        }
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

        Ok(Rule::Choice(rules))
    }

    fn rule_single(&mut self) -> Result<Rule> {
        // Prefix + rule
        let mut rule = match &self.token {
            Token::And => todo!(),
            Token::Not => {
                self.consume(Token::Not)?;
                let rule = self.rule()?;
                Rule::Not(Box::new(rule))
            }
            Token::LParen => {
                self.consume(Token::LParen)?;
                let rule = self.rule()?;
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
                Rule::Literal(literal)
            }
            Token::Text(_) => {
                let text = self.text()?;
                Rule::Ref(text)
            }
            _ => bail!("Unexpected token {:?} while parsing rule", self.token),
        };

        // postfix
        match self.token {
            Token::ZeroOrMore => {
                self.consume(Token::ZeroOrMore)?;
                rule = Rule::Repetion(0, u32::MAX, Box::new(rule))
            }
            Token::OneOrMore => {
                self.consume(Token::OneOrMore)?;
                rule = Rule::Repetion(1, u32::MAX, Box::new(rule))
            }
            Token::Optional => {
                self.consume(Token::Optional)?;
                rule = Rule::Repetion(0, 1, Box::new(rule));
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
    use crate::input::StringInput;

    fn print_rules(map: HashMap<String, Rule>) -> String {
        let mut result = String::new();
        let mut rules: Vec<(String, Rule)> = map.into_iter().collect();
        rules.sort_by(|(a, _), (b, _)| a.cmp(b));

        for (name, rule) in rules {
            let srule = format!("{}: {};\n", name, print_rule(&rule));
            result.push_str(&srule);
        }
        result
    }

    fn print_rule(rule: &Rule) -> String {
        match rule {
            Rule::Literal(l) => format!("\"{}\"", l),
            Rule::Repetion(n, m, r) => {
                let mark = match (n, m) {
                    (0, 1) => "?",
                    (0, _) => "*",
                    (1, _) => "+",
                    _ => unreachable!("no such repetition"),
                };
                format!("({}){}", print_rule(r), mark)
            }
            Rule::Choice(choices) => {
                let mut result = String::new();
                for (i, choice) in choices.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" | ");
                    }

                    result.push_str(&print_rule(choice));
                }

                result
            }
            Rule::Sequence(seq) => {
                let mut result = String::new();
                for (i, choice) in seq.iter().enumerate() {
                    if i != 0 {
                        result.push_str(" ");
                    }

                    result.push_str(&print_rule(choice));
                }

                result
            }
            Rule::Not(r) => format!("!({})", print_rule(r)),
            Rule::And(r) => format!("&({})", print_rule(r)),
            Rule::Ref(r) => r.clone(),
        }
    }

    #[test]
    fn calc() {
        let peg = include_str!("../../pegs/calc.peg");
        let input = StringInput::new(peg);
        match Parser::parse(input) {
            Ok(rules) => println!("==== Created rules ====\n{}", print_rules(rules)),
            Err(e) => println!("Failed to create rules: {e}"),
        }
    }
}
