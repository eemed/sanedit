use std::sync::Arc;

use unicode_segmentation::UnicodeSegmentation;

use crate::{editor::Editor, server::ClientId};

use super::completion::Completion;

pub(crate) type PromptAction = Box<dyn FnOnce(&mut Editor, ClientId, &str) + Send + Sync>;

pub(crate) struct Prompt {
    message: String,
    input: String,
    cursor: usize,
    completion: Completion,

    /// Callback called on confirm
    on_confirm: PromptAction,
}

impl Prompt {
    pub fn new(message: &str, on_confirm: PromptAction, must_complete: bool) -> Prompt {
        Prompt {
            message: String::from(message),
            input: String::new(),
            cursor: 0,
            completion: Completion::new(must_complete),
            on_confirm,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

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

    pub fn remove_grapheme_after_cursor(&mut self) {
        let end = self.cursor;
        self.prev_grapheme();
        let start = self.cursor;
        self.input.replace_range(start..end, "");
        self.completion.match_options(&self.input);
    }

    pub fn next_completion(&mut self) {
        self.completion.select_next();
    }

    pub fn prev_completion(&mut self) {
        self.completion.select_prev();
    }

    pub fn execute_action(self, editor: &mut Editor, id: ClientId) {
        let input = self
            .selected()
            .map(|(_, item)| item.to_string())
            .unwrap_or(self.input);
        (self.on_confirm)(editor, id, &input)
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_at_cursor(&mut self, string: &str) {
        self.input.insert_str(self.cursor, string);
        self.cursor += string.len();
        self.completion.match_options(&self.input);
    }

    pub fn insert_char_at_cursor(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.completion.match_options(&self.input);
    }

    pub fn provide_completions(&mut self, completions: Vec<String>) {
        self.completion.provide_options(completions);
        self.completion.match_options(&self.input);
    }

    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&str> {
        self.completion.matches_window(count, offset)
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        self.completion.selected()
    }

    pub fn selected_pos(&self) -> Option<usize> {
        let (pos, _) = self.completion.selected()?;
        Some(pos)
    }
}

impl Default for Prompt {
    fn default() -> Self {
        Prompt::new("", Box::new(|_, _, _| {}), false)
    }
}

impl std::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prompt")
            .field("message", &self.message)
            .field("input", &self.input)
            .field("cursor", &self.cursor)
            .field("completions", &self.completion)
            .finish_non_exhaustive()
    }
}
