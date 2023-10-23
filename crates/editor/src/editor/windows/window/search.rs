use std::{num::NonZeroUsize, ops::Range};

use crate::{
    editor::{keymap::Keymap, Editor},
    server::ClientId,
};

use super::{
    prompt::{PromptAction, PromptBuilder},
    Prompt,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    #[default]
    Forward,
    Backward,
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

pub(crate) struct SearchBuilder {
    prompt: PromptBuilder,
    direction: SearchDirection,
}

impl SearchBuilder {
    pub fn new() -> SearchBuilder {
        let prompt = Prompt::builder().keymap(Keymap::search());
        SearchBuilder {
            prompt,
            direction: SearchDirection::Forward,
        }
    }

    pub fn backward(mut self) -> Self {
        self.direction = SearchDirection::Backward;
        self
    }

    pub fn prompt(mut self, msg: &str) -> Self {
        self.prompt = self.prompt.prompt(msg);
        self
    }

    pub fn on_input<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.prompt = self.prompt.on_input(fun);
        self
    }

    pub fn on_abort<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.prompt = self.prompt.on_abort(fun);
        self
    }

    pub fn on_confirm<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.prompt = self.prompt.on_confirm(fun);
        self
    }

    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.prompt = self.prompt.keymap(keymap);
        self
    }

    pub fn history_size(mut self, size: NonZeroUsize) -> Self {
        self.prompt = self.prompt.history_size(size);
        self
    }

    pub fn build(mut self) -> Search {
        let SearchBuilder { prompt, direction } = self;

        Search {
            prompt: prompt.build(),
            hl_matches: vec![],
            cmatch: None,
            direction,
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

    pub fn builder() -> SearchBuilder {
        SearchBuilder::new()
    }

    pub fn on_confirm(&self) -> Option<PromptAction> {
        self.prompt.on_confirm()
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
