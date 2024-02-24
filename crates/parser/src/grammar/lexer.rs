use std::collections::VecDeque;

use anyhow::bail;
use anyhow::Result;

use crate::input::Input;
use crate::input::Position;

#[derive(Debug, PartialEq, Eq, Clone)]
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
    EOF,
    End,
}

pub(crate) struct Lexer<I: Input> {
    input: I,
    queue: VecDeque<Token>,
    token_count: usize,
}

impl<I: Input> Lexer<I> {
    pub fn new(input: I) -> Lexer<I> {
        Self {
            input,
            queue: VecDeque::new(),
            token_count: 0,
        }
    }

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

    pub fn pos(&self) -> Position {
        self.input.pos()
    }

    pub fn token_count(&self) -> usize {
        self.token_count
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
            } else {
                break;
            }

            self.input.consume(ch)?;
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

    fn consume_string_until(&mut self, until: char) -> Result<String> {
        let mut result = String::new();
        loop {
            let ch = match self.input.peek() {
                Some(ch) => ch,
                None => bail!("Failed to consume string at {}", self.input.pos()),
            };

            if ch == until {
                self.advance()?;
                break;
            }

            self.advance()?;
            result.push(ch);
        }

        if result.is_empty() {
            bail!("Failed to parse string at {}", self.input.pos());
        }

        Ok(result)
    }

    pub fn next(&mut self) -> Result<Token> {
        let tok = self.next_impl()?;
        self.token_count += 1;
        Ok(tok)
    }

    fn next_impl(&mut self) -> Result<Token> {
        if !self.queue.is_empty() {
            return Ok(self.queue.pop_front().unwrap());
        }

        self.skip_whitespace()?;

        let ch = match self.input.peek() {
            Some(ch) => ch,
            None => return Ok(Token::EOF),
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
            '+' => {
                self.advance()?;
                return Ok(Token::OneOrMore);
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
                let string = self.consume_string_until('"')?;
                self.queue.push_back(Token::Text(string));
                self.queue.push_back(Token::Quote);
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
            ';' => {
                self.advance()?;
                return Ok(Token::End);
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
