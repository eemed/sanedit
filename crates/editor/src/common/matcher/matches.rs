use std::{path::PathBuf, sync::Arc};

use sanedit_core::Range;
use sanedit_lsp::CompletionItem;

use crate::editor::snippets::{Snippet, SNIPPET_DESCRIPTION};

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub(crate) enum Choice {
    Snippet {
        display: String,
        snippet: Snippet,
    },
    Path {
        display: String,
        path: PathBuf,
    },
    Text {
        text: String,
        description: String,
    },
    Numbered {
        n: usize,
        text: String,
        display: String,
    },
    LSPCompletion {
        item: Box<CompletionItem>,
    },
}

impl Choice {
    pub fn from_completion_item(completion: CompletionItem) -> Arc<Choice> {
        Arc::new(Choice::LSPCompletion {
            item: Box::new(completion),
        })
    }

    pub fn from_numbered_text(n: usize, text: String) -> Arc<Choice> {
        Arc::new(Choice::Numbered {
            display: format!("{}: {}", n, text),
            n,
            text: text.into(),
        })
    }

    pub fn from_text(text: String) -> Arc<Choice> {
        Arc::new(Choice::Text {
            text: text.into(),
            description: String::new(),
        })
    }

    pub fn from_text_with_description(text: String, desc: String) -> Arc<Choice> {
        Arc::new(Choice::Text {
            text: text.into(),
            description: desc.into(),
        })
    }

    pub fn from_path(path: PathBuf, strip: usize) -> Arc<Choice> {
        let display = path.to_string_lossy();
        let display = display[strip..].to_string();
        Arc::new(Choice::Path { path, display })
    }

    pub fn from_snippet_trigger(snippet: Snippet) -> Arc<Choice> {
        Arc::new(Choice::Snippet {
            display: snippet.trigger().into(),
            snippet,
        })
    }

    /// Text used to show this option
    pub fn text(&self) -> &str {
        match self {
            Choice::Snippet { display, .. } => display.as_str(),
            Choice::Path { display, .. } => display.as_str(),
            Choice::Text { text, .. } => text.as_str(),
            Choice::Numbered { display, .. } => display.as_str(),
            Choice::LSPCompletion { item } => item.text.as_str(),
        }
    }

    /// Text used to filter / match this option
    pub fn filter_text(&self) -> &str {
        match self {
            Choice::LSPCompletion { item } => item.filter_text(),
            _ => self.text(),
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Choice::Snippet { .. } => SNIPPET_DESCRIPTION,
            Choice::Text { description, .. } => description,
            Choice::LSPCompletion { item } => item.description(),
            _ => "",
        }
    }

    pub fn number(&self) -> Option<usize> {
        match self {
            Choice::Numbered { n, .. } => Some(*n),
            _ => None,
        }
    }
}

#[derive(Debug, Eq, Ord, PartialOrd, Clone)]
pub(crate) struct ScoredChoice {
    score: usize,
    matches: Vec<Range<usize>>,
    choice: Arc<Choice>,
}

impl PartialEq for ScoredChoice {
    fn eq(&self, other: &Self) -> bool {
        (&self.score, &self.matches, &self.choice).eq(&(
            &other.score,
            &other.matches,
            &other.choice,
        ))
    }
}

impl ScoredChoice {
    pub fn new(choice: Arc<Choice>, score: usize, matches: Vec<Range<usize>>) -> ScoredChoice {
        ScoredChoice {
            score,
            matches,
            choice,
        }
    }

    pub fn rescore(&mut self, score: usize) {
        self.score = score;
    }

    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    pub fn score(&self) -> usize {
        self.score
    }

    pub fn choice(&self) -> &Choice {
        self.choice.as_ref()
    }
}
