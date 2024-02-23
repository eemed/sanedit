use anyhow::bail;
use anyhow::Result;

use crate::input::Input;

pub(crate) enum Token {
    Assign,
    ZeroOrMore,
    OneOrMore,
    Optional,
    Choice,
    And,
    Not,
    Text(String),
    LParen,
    RParen,
    Quote,
}

pub(crate) struct Lexer<I: Input> {
    input: I,
}

impl<I: Input> Lexer<I> {
    fn skip_whitespace(&mut self) -> Result<()> {
        while let Some(ch) = self.input.peek() {
            if ch.is_whitespace() {
                self.input.consume(ch)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn consume(&mut self, s: &str) -> Result<()> {
        let chars = s.chars();
        for ch in chars {
            self.input.consume(ch)?;
        }
        Ok(())
    }

    fn consume_string(&mut self) -> Result<String> {
        let mut result = String::new();
        while let Some(ch) = self.input.peek() {
            if ch.is_alphabetic() {
                result.push(ch);
            }
        }

        if result.is_empty() {
            bail!("Failed to parse string at {}", self.input.pos());
        }

        Ok(result)
    }

    fn advance(&mut self) -> Result<()> {
        if let Some(ch) = self.input.peek() {
            self.input.consume(ch)?;
            Ok(())
        } else {
            bail!("Failed to advance at {}", self.input.pos());
        }
    }

    pub fn next(&mut self) -> Result<Token> {
        self.skip_whitespace()?;

        let ch = match self.input.peek() {
            Some(ch) => ch,
            None => bail!("Unexpected end of input"),
        };

        match ch {
            '<' => {
                self.consume("<-")?;
                return Ok(Token::Assign);
            }
            '*' => {
                self.advance()?;
                return Ok(Token::ZeroOrMore);
            }
            '?' => {
                self.advance()?;
                return Ok(Token::Optional);
            }
            '|' => {
                self.advance()?;
                return Ok(Token::Choice);
            }
            '&' => {
                self.advance()?;
                return Ok(Token::And);
            }
            '!' => {
                self.advance()?;
                return Ok(Token::Not);
            }
            '"' => {
                self.advance()?;
                return Ok(Token::Quote);
            }
            '(' => {
                self.advance()?;
                return Ok(Token::LParen);
            }
            ')' => {
                self.advance()?;
                return Ok(Token::RParen);
            }
            _ => {
                if ch.is_alphabetic() {
                    let string = self.consume_string()?;
                    return Ok(Token::Text(string));
                }
            }
        }

        bail!("Unexpected char {ch} at {}", self.input.pos());
    }
}
