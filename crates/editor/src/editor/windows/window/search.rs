use crate::editor::keymap::Keymap;

use super::{Prompt, PromptAction};

#[derive(Debug)]
pub(crate) struct Search {
    prompt: Prompt,

    /// Wether to search using regex or not
    is_regex: bool,

    /// Wether to select the matches or not
    select: bool,
}

impl Search {
    pub fn new(msg: &str) -> Search {
        let mut prompt = Prompt::new(msg);
        prompt.keymap = Keymap::default_search();

        Search {
            prompt,
            is_regex: true,
            select: true,
        }
    }

    pub fn on_confirm(mut self, action: PromptAction) -> Self {
        self.prompt = self.prompt.on_confirm(action);
        self
    }

    pub fn on_abort(mut self, action: PromptAction) -> Self {
        self.prompt = self.prompt.on_abort(action);
        self
    }

    pub fn on_input(mut self, action: PromptAction) -> Self {
        self.prompt = self.prompt.on_input(action);
        self
    }

    pub fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    pub fn prompt_mut(&mut self) -> &mut Prompt {
        &mut self.prompt
    }

    pub fn keymap(&self) -> &Keymap {
        self.prompt.keymap()
    }
}

impl From<Search> for Prompt {
    fn from(search: Search) -> Self {
        search.prompt
    }
}
