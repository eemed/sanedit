use std::io;

use anyhow::bail;
use anyhow::Result;

use super::reader::Position;
use super::reader::Reader;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Token {
    Annotation,
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
    LBracket,
    RBracket,
    Char(char),
    Byte(u8),
    Range,
    Any,
    Negate,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    Quote,
    Bracket,
}

#[derive(Debug)]
pub(crate) struct Lexer<R: io::Read> {
    read: Reader<R>,
    state: State,
    token_count: usize,
}

impl<R: io::Read> Lexer<R> {
    pub fn new(input: R) -> Lexer<R> {
        Self {
            read: Reader::new(input),
            state: State::Normal,
            token_count: 0,
        }
    }

    fn skip_whitespace(&mut self) -> Result<()> {
        if self.state == State::Bracket {
            return Ok(());
        }

        while let Some(ch) = self.read.peek() {
            if ch.is_whitespace() {
                self.read.consume(ch)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    pub fn pos(&self) -> Position {
        self.read.pos()
    }

    pub fn token_count(&self) -> usize {
        self.token_count
    }

    fn consume(&mut self, s: &str) -> Result<()> {
        let chars = s.chars();
        for ch in chars {
            self.read.consume(ch)?;
        }
        Ok(())
    }

    fn consume_string(&mut self) -> Result<String> {
        let mut result = String::new();
        while let Some(ch) = self.read.peek() {
            if Self::allowed_in_string(ch) {
                result.push(ch);
            } else {
                break;
            }

            self.read.consume(ch)?;
        }

        if result.is_empty() {
            bail!("Failed to parse string at {}", self.read.pos());
        }

        Ok(result)
    }

    fn consume_hex(&mut self) -> Result<String> {
        let mut result = String::new();
        while let Some(ch) = self.read.peek() {
            if ch.is_ascii_hexdigit() {
                result.push(ch);
            } else {
                break;
            }

            self.read.consume(ch)?;
        }

        if result.is_empty() {
            bail!("Failed to parse string at {}", self.read.pos());
        }

        Ok(result)
    }

    fn advance(&mut self) -> Result<()> {
        if let Some(ch) = self.read.peek() {
            self.read.consume(ch)?;
            Ok(())
        } else {
            bail!("Failed to advance at {}", self.read.pos());
        }
    }

    fn consume_string_until(&mut self, until: char) -> Result<String> {
        let mut result = String::new();
        let mut prev_escape = false;
        loop {
            let ch = match self.read.peek() {
                Some(ch) => ch,
                None => bail!("Failed to consume string at {}", self.read.pos()),
            };
            let escape = !prev_escape && ch == '\\';

            if !prev_escape && ch == until {
                break;
            }

            self.advance()?;
            if !escape || prev_escape {
                if prev_escape {
                    match ch {
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        'n' => result.push('\n'),
                        _ => result.push(ch),
                    }
                } else {
                    result.push(ch)
                }
            }

            prev_escape = escape;
        }

        if result.is_empty() {
            bail!("Failed to parse string at {}", self.read.pos());
        }

        Ok(result)
    }

    pub fn next(&mut self) -> Result<Token> {
        self.skip_whitespace()?;

        let tok = match self.state {
            State::Normal => self.normal()?,
            State::Quote => self.quote()?,
            State::Bracket => self.brackets()?,
        };

        self.token_count += 1;
        Ok(tok)
    }

    fn normal(&mut self) -> Result<Token> {
        let ch = match self.read.peek() {
            Some(ch) => ch,
            None => return Ok(Token::EOF),
        };

        match ch {
            '.' => {
                self.advance()?;
                return Ok(Token::Any);
            }
            '@' => {
                self.advance()?;
                return Ok(Token::Annotation);
            }
            '=' => {
                self.advance()?;
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
            '/' => {
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
                self.state = State::Quote;
                return Ok(Token::Quote);
            }
            '[' => {
                self.advance()?;
                self.state = State::Bracket;
                return Ok(Token::LBracket);
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
            '#' => {
                self.consume_string_until('\n')?;
                self.advance()?;
                return self.next();
            }
            c => {
                if Self::allowed_in_string(c) {
                    let string = self.consume_string()?;
                    return Ok(Token::Text(string));
                }
            }
        }

        bail!("Unexpected char {ch} at {}", self.read.pos());
    }

    fn allowed_in_string(ch: char) -> bool {
        let allowed = ['-', '_'];
        ch.is_alphabetic() || allowed.contains(&ch)
    }

    fn quote(&mut self) -> Result<Token> {
        let ch = match self.read.peek() {
            Some(ch) => ch,
            None => return Ok(Token::EOF),
        };

        match ch {
            '"' => {
                self.advance()?;
                self.state = State::Normal;
                return Ok(Token::Quote);
            }
            _ => {
                let string = self.consume_string_until('"')?;
                return Ok(Token::Text(string));
            }
        }
    }

    fn brackets(&mut self) -> Result<Token> {
        let ch = match self.read.peek() {
            Some(ch) => ch,
            None => return Ok(Token::EOF),
        };

        match ch {
            ']' => {
                self.advance()?;
                self.state = State::Normal;
                return Ok(Token::RBracket);
            }
            '^' => {
                self.advance()?;
                return Ok(Token::Negate);
            }
            '\\' => {
                self.advance()?;
                match self.read.peek() {
                    Some('n') => {
                        self.advance()?;
                        Ok(Token::Char('\n'))
                    }
                    Some('t') => {
                        self.advance()?;
                        Ok(Token::Char('\t'))
                    }
                    Some('r') => {
                        self.advance()?;
                        Ok(Token::Char('\r'))
                    }
                    Some(']') => {
                        self.advance()?;
                        Ok(Token::Char(']'))
                    }
                    Some('.') => {
                        self.advance()?;
                        Ok(Token::Char('.'))
                    }
                    Some('[') => {
                        self.advance()?;
                        Ok(Token::Char('['))
                    }
                    Some('^') => {
                        self.advance()?;
                        Ok(Token::Char('^'))
                    }
                    Some('\\') => {
                        self.advance()?;
                        Ok(Token::Char('\\'))
                    }
                    Some('u') => {
                        self.advance()?;
                        let hex = self.consume_hex()?;
                        if hex.len() != 4 {
                            bail!("Unicode escape needs to have 4 hex digits");
                        }
                        let num = u32::from_str_radix(&hex, 16)?;
                        match char::from_u32(num) {
                            Some(c) => Ok(Token::Char(c)),
                            None => bail!("Cannot convert unicode {hex} to char"),
                        }
                    }
                    Some('x') => {
                        self.advance()?;
                        let hex = self.consume_hex()?;
                        if hex.len() > 2 {
                            bail!("Hex only upto \\xff allowed");
                        }
                        let num = u32::from_str_radix(&hex, 16)?;
                        if num < u8::MAX as u32 {
                            Ok(Token::Byte(num as u8))
                        } else {
                            bail!("Cannot convert hex {hex} to char");
                        }
                    }
                    Some(c) => bail!("Expected escaped char but got {c}, at {}", self.read.pos()),
                    None => bail!(
                        "Expected escaped char but got Nothing, at {}",
                        self.read.pos()
                    ),
                }
            }
            '.' => {
                self.advance()?;

                if let Some(n) = self.read.peek() {
                    if n == '.' {
                        self.advance()?;
                        return Ok(Token::Range);
                    }
                }

                Ok(Token::Char('.'))
            }
            ch => {
                self.advance()?;
                return Ok(Token::Char(ch));
            }
        }
    }
}
