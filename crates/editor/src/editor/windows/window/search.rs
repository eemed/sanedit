use crate::editor::keymap::Keymap;

use super::{PAction, Prompt, SetPrompt};

pub(crate) struct SetSearch {
    pub prompt: SetPrompt,

    pub is_regex: bool,
    pub select: bool,
    pub stop_at_first_match: bool,
}

#[derive(Debug)]
pub(crate) struct Search {
    pub prompt: Prompt,

    /// Wether to search using regex or not
    is_regex: bool,

    /// Wether to select the matches or not
    select: bool,
    stop_at_first_match: bool,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        let mut prompt = Prompt::new(msg);
        prompt.keymap = Keymap::search();

        Search {
            prompt,
            is_regex: true,
            select: true,
            stop_at_first_match: true,
        }
    }

    pub fn set(&mut self, set: SetSearch) {
        let SetSearch {
            prompt,
            is_regex,
            select,
            stop_at_first_match,
        } = set;

        self.prompt.set(prompt);
        self.is_regex = is_regex;
        self.select = select;
        self.stop_at_first_match = stop_at_first_match;
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
