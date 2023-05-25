use std::ops::Range;

use crate::editor::keymap::Keymap;

use super::{Prompt, SetPrompt};

pub(crate) struct SetSearch {
    pub prompt: SetPrompt,
    // pub is_regex: bool,
    pub select: bool,
    pub stop_at_first_match: bool,
}

#[derive(Debug)]
pub(crate) struct Search {
    pub prompt: Prompt,
    pub matches: Vec<Range<usize>>,

    // /// Wether to search using regex or not
    // pub is_regex: bool,
    // pub is_valid_regex: bool,
    /// Wether to select the matches or not
    pub select: bool,
    pub stop_at_first_match: bool,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        let mut prompt = Prompt::new(msg);
        prompt.keymap = Keymap::search();

        Search {
            prompt,
            matches: vec![],
            select: false,
            stop_at_first_match: true,
        }
    }

    pub fn set(&mut self, set: SetSearch) {
        let SetSearch {
            prompt,
            select,
            stop_at_first_match,
        } = set;

        self.prompt.set(prompt);
        self.select = select;
        self.stop_at_first_match = stop_at_first_match;
    }

    pub fn select(&self) -> bool {
        self.select
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
