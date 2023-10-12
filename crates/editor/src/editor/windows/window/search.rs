use std::ops::Range;

use crate::{
    editor::{keymap::Keymap, Editor},
    server::ClientId,
};

use super::{prompt::PromptAction, Prompt};

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
    pub hl_matches: Vec<Range<usize>>,
    pub cmatch: Option<Range<usize>>,

    pub direction: SearchDirection,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        let mut prompt = Prompt::new(msg);
        prompt.keymap = Keymap::search();

        Search {
            prompt,
            hl_matches: vec![],
            cmatch: None,
            direction: SearchDirection::Forward,
        }
    }

    pub fn on_confirm<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.prompt = self.prompt.on_confirm(fun);
        self
    }

    pub fn get_on_confirm(&self) -> Option<PromptAction> {
        self.prompt.get_on_confirm()
    }

    pub fn save_to_history(&mut self) {
        self.prompt.save_to_history();
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
