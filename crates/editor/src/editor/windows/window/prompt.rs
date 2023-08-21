mod history;

use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    common::matcher::Match,
    editor::{keymap::Keymap, Editor},
    server::ClientId,
};

use self::history::History;

use super::completion::Selector;

/// Prompt action, similar to a normal `ActionFunction` but also takes the
/// prompt input as a additional parameter
pub(crate) type PromptAction = Rc<dyn Fn(&mut Editor, ClientId, &str) + Send + Sync>;

pub(crate) struct Prompt {
    pub message: String,

    input: String,
    cursor: usize,
    selector: Selector,

    /// Called when prompt is confirmed
    pub on_confirm: Option<PromptAction>,

    /// Called when prompt is aborted
    pub on_abort: Option<PromptAction>,

    /// Called when input is modified
    pub on_input: Option<PromptAction>,
    pub keymap: Keymap,

    pub history: History,
}

impl Prompt {
    pub fn new(message: &str) -> Prompt {
        Prompt {
            message: String::from(message),
            input: String::new(),
            cursor: 0,
            selector: Selector::new(),
            on_confirm: None,
            on_abort: None,
            on_input: None,
            keymap: Keymap::prompt(),
            history: History::new(100),
        }
    }

    pub fn reset_selector(&mut self) {
        self.selector = Selector::new();
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

    pub fn remove_grapheme_before_cursor(&mut self) {
        let end = self.cursor;
        self.prev_grapheme();
        let start = self.cursor;
        self.input.replace_range(start..end, "");
    }

    pub fn next_completion(&mut self) {
        self.selector.select_next();
    }

    pub fn prev_completion(&mut self) {
        self.selector.select_prev();
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn input_or_selected(&self) -> String {
        self.selected()
            .map(|(_, item)| item.to_string())
            .unwrap_or(self.input.clone())
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_at_cursor(&mut self, string: &str) {
        self.input.insert_str(self.cursor, string);
        self.cursor += string.len();
    }

    pub fn insert_char_at_cursor(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn provide_completions(&mut self, completions: Vec<Match>) {
        self.selector.provide_options(completions);
    }

    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&str> {
        self.selector.matches_window(count, offset)
    }

    pub fn selected(&self) -> Option<(usize, &str)> {
        self.selector.selected()
    }

    pub fn selected_pos(&self) -> Option<usize> {
        let (pos, _) = self.selector.selected()?;
        Some(pos)
    }

    pub fn history_next(&mut self) {
        match self.history.next() {
            Some(item) => {
                self.cursor = item.len();
                self.input = item.into();
            }
            None => {
                self.cursor = 0;
                self.input = String::new();
            }
        }
    }

    pub fn history_prev(&mut self) {
        match self.history.prev() {
            Some(item) => {
                self.cursor = item.len();
                self.input = item.into();
            }
            None => {
                self.cursor = 0;
                self.input = String::new();
            }
        }
    }
}

impl Default for Prompt {
    fn default() -> Self {
        Prompt::new("")
    }
}

impl std::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prompt")
            .field("message", &self.message)
            .field("input", &self.input)
            .field("cursor", &self.cursor)
            .field("completions", &self.selector)
            .finish_non_exhaustive()
    }
}
