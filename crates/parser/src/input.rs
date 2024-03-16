use std::{
    fmt,
    io::{self, BufReader, Read},
    iter::Peekable,
    str::Chars,
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

#[derive(Debug)]
pub(crate) struct Reader<T: io::Read> {
    pos: Position,
    next: Option<char>,
    read: BufReader<T>,
}

impl<T: io::Read> Reader<T> {
    pub fn new(read: T) -> Reader<T> {
        let mut me = Reader {
            pos: Position { line: 0, col: 0 },
            next: None,
            read: BufReader::new(read),
        };
        me.read_next_char();
        me
    }

    fn read_next_char(&mut self) {
        self.next = None;

        let mut buf = [0; 1];
        if let Err(_) = self.read.read_exact(&mut buf) {
            return;
        }

        let n = buf[0].leading_ones();
        match n {
            0 => self.next = char::from_u32(buf[0] as u32),
            2 | 3 | 4 => {
                let mut big = [0; 4];
                big[0] = buf[0];

                if let Err(_) = self.read.read_exact(&mut big[1..n as usize]) {
                    return;
                }
                self.next = char::from_u32(big[0] as u32);
            }
            _ => {}
        }
    }
}

impl<T: io::Read> Input for Reader<T> {
    fn peek(&mut self) -> Option<char> {
        self.next
    }

    fn consume(&mut self, ch: char) -> anyhow::Result<()> {
        match self.next {
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

                self.read_next_char();
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
