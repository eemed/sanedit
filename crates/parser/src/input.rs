use std::{fmt, iter::Peekable, str::Chars};

use anyhow::bail;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Position {
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line: {}, Col: {}", self.line, self.col)
    }
}

pub(crate) trait Input {
    fn peek(&mut self) -> Option<char>;
    fn consume(&mut self, ch: char) -> anyhow::Result<()>;
    fn pos(&self) -> Position;
}

pub(crate) struct StringInput<'a> {
    input: Peekable<Chars<'a>>,
    pos: Position,
}

impl<'a> StringInput<'a> {
    pub fn new(input: &'a str) -> StringInput<'a> {
        StringInput {
            input: input.chars().peekable(),
            pos: Position { line: 1, col: 0 },
        }
    }
}

impl<'a> Input for StringInput<'a> {
    fn peek(&mut self) -> Option<char> {
        self.input.peek().cloned()
    }

    fn consume(&mut self, ch: char) -> anyhow::Result<()> {
        match self.peek() {
            Some(got) => {
                if got != ch {
                    bail!("Tried to consume {} but was {} at {}", ch, got, self.pos());
                }

                if ch == '\n' {
                    self.pos.col = 0;
                    self.pos.line += 1;
                } else {
                    self.pos.col += 1;
                }

                self.input.next();
            }
            None => {
                bail!("Tried to consume {ch} but input ended at {}", self.pos());
            }
        }

        Ok(())
    }

    fn pos(&self) -> Position {
        self.pos
    }
}
