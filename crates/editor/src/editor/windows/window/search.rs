use std::ops::Range;

use crate::editor::keymap::Keymap;

use super::Prompt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    Backward,
    Forward,
}

impl SearchDirection {
    pub fn opposite(&self) -> SearchDirection {
        use SearchDirection::*;
        match self {
            Backward => Forward,
            Forward => Backward,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Search {
    pub prompt: Prompt,
    pub cmatch: Option<Range<usize>>,

    pub direction: SearchDirection,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        let mut prompt = Prompt::new(msg);
        prompt.keymap = Keymap::search();

        Search {
            prompt,
            cmatch: None,
            direction: SearchDirection::Forward,
        }
    }
}

impl Default for Search {
    fn default() -> Self {
        Search::new("")
    }
}

impl From<Search> for Prompt {
    fn from(search: Search) -> Self {
        search.prompt
    }
}
