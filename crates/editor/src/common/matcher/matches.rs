use std::{borrow::Cow, path::PathBuf, sync::Arc};

use sanedit_core::Range;

use crate::editor::snippets::{Snippet, SNIPPET_DESCRIPTION};

#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub(crate) enum Choice {
    Snippet { snippet: Snippet, trigger: bool },
    Path { path: PathBuf, strip: usize },
    Text { text: String, description: String },
}

impl Choice {
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
        Arc::new(Choice::Path { path, strip })
    }

    pub fn from_snippet(snippet: Snippet) -> Arc<Choice> {
        Arc::new(Choice::Snippet {
            snippet,
            trigger: false,
        })
    }

    pub fn from_snippet_trigger(snippet: Snippet) -> Arc<Choice> {
        Arc::new(Choice::Snippet {
            snippet,
            trigger: true,
        })
    }

    pub fn text(&self) -> Cow<str> {
        match self {
            Choice::Snippet { snippet, trigger } => {
                if *trigger {
                    snippet.trigger().into()
                } else {
                    todo!()
                }
            }
            Choice::Path { path, strip } => {
                let path = path.to_string_lossy();
                let path = &path[*strip..];
                Cow::Owned(path.to_string())
            }
            Choice::Text { text, .. } => text.into(),
        }
    }

    pub fn description(&self) -> Cow<str> {
        match self {
            Choice::Snippet { .. } => SNIPPET_DESCRIPTION.into(),
            Choice::Path { .. } => "".into(),
            Choice::Text { description, .. } => description.into(),
        }
    }
}

#[derive(Debug, Eq, Ord, PartialOrd, Clone)]
pub(crate) struct ScoredChoice {
    score: u32,
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
    pub fn new(choice: Arc<Choice>, score: u32, matches: Vec<Range<usize>>) -> ScoredChoice {
        ScoredChoice {
            score,
            matches,
            choice,
        }
    }

    pub fn rescore(&mut self, score: u32) {
        self.score = score;
    }

    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn choice(&self) -> &Choice {
        self.choice.as_ref()
    }
}
