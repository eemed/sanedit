use std::ops::Range;

use crate::editor::keymap::Keymap;

use super::Prompt;

#[derive(Debug, Clone, Copy)]
pub(crate) enum SearchDirection {
    Backward,
    Forward,
}

#[derive(Debug)]
pub(crate) struct Search {
    pub prompt: Prompt,
    pub matches: Vec<Range<usize>>,

    pub direction: SearchDirection,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        let mut prompt = Prompt::new(msg);
        prompt.keymap = Keymap::search();

        Search {
            prompt,
            matches: vec![],
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
