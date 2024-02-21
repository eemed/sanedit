use std::{iter::Peekable, str::Chars};

use anyhow::bail;

pub(crate) trait Input {
    fn peek(&mut self) -> Option<char>;
    fn consume(&mut self, ch: char) -> anyhow::Result<()>;
    fn pos(&self) -> usize;
}

pub(crate) struct StringInput<'a> {
    input: Peekable<Chars<'a>>,
    pos: usize,
}

impl<'a> StringInput<'a> {
    pub fn new(input: &'a str) -> StringInput<'a> {
        StringInput {
            input: input.chars().peekable(),
            pos: 0,
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
                self.input.next();
                self.pos += ch.len_utf8();
            }
            None => {
                bail!("Tried to consume {ch} but input ended at {}", self.pos());
            }
        }

        Ok(())
    }

    fn pos(&self) -> usize {
        self.pos
    }
}
