use std::{path::PathBuf, sync::Arc};

use sanedit_core::{Choice, Range};

use crate::editor::snippets::{Snippet, SNIPPET_DESCRIPTION};

#[derive(Debug, Eq, Ord, PartialOrd, Clone)]
pub(crate) struct ScoredChoice {
    score: u32,
    matches: Vec<Range<usize>>,
    choice: Arc<dyn Choice>,
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
    pub fn new(choice: Arc<dyn Choice>, score: u32, matches: Vec<Range<usize>>) -> ScoredChoice {
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

    pub fn choice(&self) -> &dyn Choice {
        self.choice.as_ref()
    }
}

#[derive(Debug)]
pub(crate) struct PathChoice {
    path: PathBuf,
    as_string: String,
}

impl PathChoice {
    pub fn new(path: PathBuf, strip: usize) -> PathChoice {
        let as_string = path.to_string_lossy()[strip..].to_string();
        PathChoice { path, as_string }
    }
}

impl Choice for PathChoice {
    fn description(&self) -> &str {
        ""
    }

    fn text(&self) -> &str {
        &self.as_string
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.path
    }
}

#[derive(Debug)]
pub(crate) struct SnippetChoice {
    snippet: Snippet,
    as_string: String,
}

impl SnippetChoice {
    pub fn new(snippet: Snippet) -> SnippetChoice {
        SnippetChoice {
            snippet,
            as_string: "todo".to_string(),
        }
    }

    pub fn new_trigger(snippet: Snippet) -> SnippetChoice {
        SnippetChoice {
            as_string: snippet.trigger().to_string(),
            snippet,
        }
    }
}

impl Choice for SnippetChoice {
    fn description(&self) -> &str {
        SNIPPET_DESCRIPTION
    }

    fn text(&self) -> &str {
        &self.as_string
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.snippet
    }
}
