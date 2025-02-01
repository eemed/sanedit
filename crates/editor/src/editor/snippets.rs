use anyhow::anyhow;
use sanedit_core::{Directory, Filetype, SNIPPETS_FILE};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    iter::Peekable,
    path::{Path, PathBuf},
    str::Chars,
    sync::Arc,
};
use thiserror::Error;

use crate::common::matcher::Choice;

use super::Map;

pub(crate) const SNIPPET_DESCRIPTION: &str = "snippet";

#[derive(Debug)]
pub(crate) struct Snippets {
    filetype_dir: Directory,
    map: Map<Filetype, Map<String, Snippet>>,
    global: Map<String, Snippet>,
}

impl Snippets {
    pub fn new(global_file: &Path, ft_dir: Directory) -> Snippets {
        let mut global = Map::default();
        if global_file.exists() {
            match Self::load_snippet_file(global_file) {
                Ok(snips) => global = snips,
                Err(e) => {
                    log::error!("Failed to load global snippets from {global_file:?}: {e}");
                }
            }
        }

        Snippets {
            filetype_dir: ft_dir,
            map: Map::default(),
            global,
        }
    }

    /// all snippets available for filetype
    pub fn all(&self, ft: Option<&Filetype>) -> BTreeMap<String, &Snippet> {
        let mut snippets = BTreeMap::new();

        for (name, snippet) in &self.global {
            snippets.insert(name.clone(), snippet);
        }

        // Add/Overwrite with local
        if let Some(ft) = ft {
            if let Some(local) = self.map.get(ft) {
                for (name, snippet) in local {
                    snippets.insert(name.clone(), snippet);
                }
            }
        }

        snippets
    }

    /// Get a specific snippet for a filetype
    pub fn get_snippet(&self, ft: Option<&Filetype>, name: &str) -> Option<&Snippet> {
        if let Some(ft) = ft {
            if let Some(local) = self.map.get(ft) {
                if let Some(snip) = local.get(name) {
                    return Some(snip);
                }
            }
        }

        self.global.get(name)
    }

    pub fn load_global(&mut self, path: &Path) -> anyhow::Result<Map<String, Snippet>> {
        log::debug!("Loading global snippets from: {path:?}");
        if !path.exists() {
            return Ok(Map::default());
        }

        Self::load_snippet_file(path)
    }

    pub fn load(&mut self, ft: &Filetype) -> anyhow::Result<Map<String, Snippet>> {
        let path = PathBuf::from(ft.as_str()).join(SNIPPETS_FILE);
        let snippets = self.filetype_dir.find(&path).ok_or(anyhow!(
            "Could not find snippet file for filetype {}",
            ft.as_str()
        ))?;
        Self::load_snippet_file(snippets.as_path())
    }

    fn load_snippet_file(path: &Path) -> anyhow::Result<Map<String, Snippet>> {
        let config = config::Config::builder()
            .add_source(config::File::from(path))
            .build()?;
        let map = config.try_deserialize::<Map<String, ConfigSnippet>>()?;

        let mut snippets = Map::default();
        for (name, snip) in map {
            match Snippet::new_trigger(&snip.body, &snip.trigger) {
                Ok(snippet) => {
                    snippets.insert(name, snippet);
                }
                Err(e) => log::error!("Failed to parse snippet: {name}: {e}"),
            }
        }

        Ok(snippets)
    }

    pub fn match_options(&self, ft: Option<&Filetype>) -> Vec<Arc<Choice>> {
        self.all(ft)
            .into_iter()
            .map(|(name, snippet)| Choice::from_snippet_trigger(snippet.clone()))
            .collect()
    }
}

/// Used for parsing snippet files
#[derive(Debug, Deserialize)]
pub(crate) struct ConfigSnippet {
    trigger: String,
    body: String,
}

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

        if atoms.is_empty() {
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
