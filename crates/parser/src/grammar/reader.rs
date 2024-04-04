use std::{
    fmt,
    io::{self, BufRead, BufReader},
};

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

#[derive(Debug)]
pub(crate) struct Reader<T: io::Read> {
    pos: Position,
    read: BufReader<T>,
    line: String,
}

impl<T: io::Read> Reader<T> {
    pub fn new(read: T) -> Reader<T> {
        Reader {
            pos: Position { line: 0, col: 0 },
            read: BufReader::new(read),
            line: String::new(),
        }
    }

    fn take_next(&mut self) -> Option<char> {
        if self.line.is_empty() {
            let _ = self.read.read_line(&mut self.line);
        }

        let ch = self.line.chars().next()?;
        self.line = self.line.split_off(ch.len_utf8());
        Some(ch)
    }
}

impl<T: io::Read> Reader<T> {
    pub fn peek(&mut self) -> Option<char> {
        if self.line.is_empty() {
            let _ = self.read.read_line(&mut self.line);
        }

        self.line.chars().peekable().peek().copied()
    }

    pub fn consume(&mut self, ch: char) -> anyhow::Result<()> {
        match self.take_next() {
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
            }
            None => {
                bail!("Tried to consume {ch} but input ended at {}", self.pos());
            }
        }

        Ok(())
    }

    pub fn pos(&self) -> Position {
        self.pos
    }
}
