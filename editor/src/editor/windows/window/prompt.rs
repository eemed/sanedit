use std::sync::Arc;

use unicode_segmentation::UnicodeSegmentation;

use crate::{editor::Editor, server::ClientId};

use super::completion::Completion;

pub(crate) type PromptAction = Arc<dyn Fn(&mut Editor, ClientId, &str) + Send + Sync>;

pub(crate) struct Prompt {
    pub(crate) message: String,
    pub(crate) input: String,
    pub(crate) cursor: usize,
    pub(crate) completion: Completion,

    /// Callback called on confirm
    on_confirm: PromptAction,
}

impl Prompt {
    pub fn next_grapheme(&mut self) {
        let mut graphemes = self.input.grapheme_indices(true);
        graphemes.position(|(pos, _)| pos == self.cursor);
        self.cursor = graphemes.next().map_or(self.input.len(), |(pos, _)| pos);
    }

    pub fn prev_grapheme(&mut self) {
        let graphemes = self.input.grapheme_indices(true);

        let mut last = 0;
        for (pos, _) in graphemes {
            if pos == self.cursor {
                break;
            }

            last = pos;
        }
        self.cursor = last;
    }

    pub fn remove_grapheme_at_cursor(&mut self) {
        let end = self.cursor;
        self.prev_grapheme();
        let start = self.cursor;
        self.input.replace_range(start..end, "");
        // completion_calculate_matches(&mut prompt.completion, &prompt.userinput);
    }

    pub fn next_completion(&mut self) {
        self.completion.select_next();
    }

    pub fn prev_completion(&mut self) {
        self.completion.select_prev();
    }

    pub fn action(&self) -> &PromptAction {
        &self.on_confirm
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn insert_at_cursor(&mut self, string: &str) {
        self.input.insert_str(self.cursor, string);
        self.cursor += string.len();
        // completion_calculate_matches(&mut prompt.completion, &prompt.userinput);
    }

    pub fn insert_char_at_cursor(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        // completion_calculate_matches(&mut prompt.completion, &prompt.userinput);
    }

    pub fn provide_completions(&mut self, completions: Vec<String>) {
        self.completion.provide_options(completions);
    }
}

impl std::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prompt")
            .field("message", &self.message)
            .field("input", &self.input)
            .field("cursor", &self.cursor)
            .field("completions", &self.completion)
            .finish()
    }
}
