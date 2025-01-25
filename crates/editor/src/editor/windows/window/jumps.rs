use std::{
    collections::VecDeque,
    iter::Peekable,
    str::{Chars, FromStr},
};
use thiserror::Error;

use sanedit_buffer::Mark;

use crate::editor::buffers::BufferId;

/// jump to a position or selection in buffer
#[derive(Debug)]
pub(crate) struct Jump {
    start: Mark,
    /// If jump selects a portion of the text end is set
    end: Option<Mark>,
}

impl Jump {
    pub fn new(start: Mark, end: Option<Mark>) -> Jump {
        Jump { start, end }
    }

    pub fn start(&self) -> &Mark {
        &self.start
    }

    pub fn end(&self) -> Option<&Mark> {
        self.end.as_ref()
    }
}

/// A group of jumps meant to be used at the same time.
/// Mostly to place a cursor on each jump simultaneously
#[derive(Debug)]
pub(crate) struct JumpGroup {
    bid: BufferId,
    jumps: Vec<Jump>,
}

impl JumpGroup {
    pub fn new(id: BufferId, jumps: Vec<Jump>) -> JumpGroup {
        JumpGroup { bid: id, jumps }
    }

    pub fn jumps(&self) -> &[Jump] {
        &self.jumps
    }
}

#[derive(Debug)]
pub(crate) struct Jumps {
    jumps: VecDeque<JumpGroup>,
}

impl Jumps {
    pub fn new(groups: Vec<JumpGroup>) -> Jumps {
        let mut deque = VecDeque::with_capacity(groups.len());
        deque.extend(groups);

        Jumps { jumps: deque }
    }

    pub fn next(&mut self) -> Option<JumpGroup> {
        self.jumps.pop_front()
    }
}

#[derive(Debug)]
pub(crate) enum SnippetAtom {
    Text(String),
    Placeholder(u8, String),
    Newline,
    Indent,
}

#[derive(Debug)]
pub(crate) struct Snippet {
    atoms: Vec<SnippetAtom>,
}

impl Snippet {
    pub fn new(snip: &str) -> Result<Snippet, SnippetError> {
        let mut atoms = vec![];
        let mut escaped = false;
        let mut text = String::new();
        let mut chars = snip.chars().peekable();

        while let Some(ch) = chars.next() {
            if escaped {
                escaped = false;

                match ch {
                    'n' => Self::push(&mut atoms, &mut text, SnippetAtom::Newline),
                    't' => Self::push(&mut atoms, &mut text, SnippetAtom::Indent),
                    _ => text.push(ch),
                }
            } else {
                match ch {
                    '$' => {
                        let atom = Self::parse_placeholder(&mut chars)?;
                        Self::push(&mut atoms, &mut text, atom);
                    }
                    '\\' => escaped = true,
                    '\n' => Self::push(&mut atoms, &mut text, SnippetAtom::Newline),
                    '\t' => Self::push(&mut atoms, &mut text, SnippetAtom::Indent),
                    _ => text.push(ch),
                }
            }
        }

        if atoms.is_empty() {
            return Err(SnippetError::Empty);
        }

        Ok(Snippet { atoms })
    }

    fn push(atoms: &mut Vec<SnippetAtom>, text: &mut String, atom: SnippetAtom) {
        let text = std::mem::take(text);
        if !text.is_empty() {
            atoms.push(SnippetAtom::Text(text))
        }

        atoms.push(atom);
    }

    fn parse_placeholder(chars: &mut Peekable<Chars>) -> Result<SnippetAtom, SnippetError> {
        let is_open = chars.peek().map(|ch| *ch == '{').unwrap_or(false);

        if is_open {
            // Case: ${0:foo}
            // {
            chars.next();

            // number
            let num = Self::parse_num(chars)?;

            // :
            if chars.next() != Some(':') {
                return Err(SnippetError::FailedToParsePlaceholderColon);
            }

            // name}
            let mut name = String::new();
            while let Some(ch) = chars.next() {
                if ch == '}' {
                    break;
                }

                name.push(ch);
            }

            Ok(SnippetAtom::Placeholder(num, name))
        } else {
            // Case: $0
            let num = Self::parse_num(chars)?;
            Ok(SnippetAtom::Placeholder(num, String::new()))
        }
    }

    fn parse_num(chars: &mut Peekable<Chars>) -> Result<u8, SnippetError> {
        let mut num = String::new();
        while let Some(ch) = chars.peek() {
            if !ch.is_digit(10) {
                break;
            }

            num.push(*ch);
            chars.next();
        }

        if num.is_empty() {
            return Err(SnippetError::NumberParseError);
        }

        let num = num.parse::<u8>().unwrap();
        Ok(num)
    }

    pub fn atoms(&self) -> &[SnippetAtom] {
        &self.atoms
    }
}

#[derive(Debug, Error)]
pub(crate) enum SnippetError {
    #[error("Nothing to parse")]
    Empty,

    #[error("Failed to parse placehold number")]
    NumberParseError,

    #[error("Failed to parse placeholder colon")]
    FailedToParsePlaceholderColon,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_snippet() {
        let text = "line 1\\n\\tline2 $0\\nline3 ${3:shitter}\nline4 ${3:worse}";
        let snip = dbg!(Snippet::new(text));
    }
}
