use std::{iter::Peekable, str::Chars, sync::Arc};
use thiserror::Error;

pub(crate) const SNIPPET_DESCRIPTION: &str = "snippet";

/// A snippet consists of a list of atoms
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SnippetAtom {
    Text(String),
    Placeholder(u8, String),
    Newline,
    Indent,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Snippet(Arc<SnippetInner>);

impl Snippet {
    pub fn new(snip: &str) -> Result<Snippet, SnippetError> {
        let inner = SnippetInner::new(snip)?;
        Ok(Snippet(Arc::new(inner)))
    }

    pub fn new_trigger(snip: &str, trigger: &str) -> Result<Snippet, SnippetError> {
        let inner = SnippetInner::new_trigger(snip, trigger)?;
        Ok(Snippet(Arc::new(inner)))
    }

    pub fn atoms(&self) -> &[SnippetAtom] {
        &self.0.atoms
    }

    pub fn trigger(&self) -> &str {
        &self.0.trigger
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SnippetInner {
    trigger: String,
    atoms: Vec<SnippetAtom>,
}

impl SnippetInner {
    pub fn new(snip: &str) -> Result<SnippetInner, SnippetError> {
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

        if !text.is_empty() {
            atoms.push(SnippetAtom::Text(text))
        }

        if atoms.is_empty() {
            log::error!("snip: {snip:?}");
            return Err(SnippetError::Empty);
        }

        Ok(SnippetInner {
            trigger: String::new(),
            atoms,
        })
    }

    pub fn new_trigger(snip: &str, trigger: &str) -> Result<SnippetInner, SnippetError> {
        let mut snip = Self::new(snip)?;
        snip.trigger = trigger.into();
        Ok(snip)
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
            // Case: ${0}

            // {
            chars.next();

            // number
            let num = Self::parse_num(chars)?;

            // End of case: ${0}
            if chars.peek() == Some(&'}') {
                chars.next();
                return Ok(SnippetAtom::Placeholder(num, String::new()));
            }

            // :
            if chars.next() != Some(':') {
                return Err(SnippetError::FailedToParsePlaceholderColon);
            }

            // name}
            let mut name = String::new();
            for ch in chars.by_ref() {
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
            if !ch.is_ascii_digit() {
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
        let _snip = Snippet::new(text).unwrap();
    }
}
