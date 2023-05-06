use std::{collections::VecDeque, rc::Rc};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    editor::{keymap::Keymap, Editor},
    server::ClientId,
};

use super::completion::Completion;

pub(crate) struct SetPrompt {
    pub(crate) message: String,
    pub(crate) on_confirm: Option<PAction>,
    pub(crate) on_abort: Option<PAction>,
    pub(crate) on_input: Option<PAction>,
    pub(crate) keymap: Option<Keymap>,
}

pub(crate) type PAction = Rc<dyn Fn(&mut Editor, ClientId, &str) + Send + Sync>;

#[derive(Debug, Clone, Copy)]
enum Pos {
    First,
    Last,
    Index(usize),
}

pub(crate) struct History {
    items: VecDeque<String>,
    limit: usize,
    pos: Pos,
}

impl History {
    pub fn new(limit: usize) -> History {
        History {
            items: VecDeque::with_capacity(limit),
            limit,
            pos: Pos::First,
        }
    }

    pub fn reset(&mut self) {
        self.pos = Pos::First;
    }

    pub fn get(&self) -> Option<&str> {
        match self.pos {
            Pos::Index(n) => self.items.get(n).map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn push(&mut self, item: &str) {
        self.items.retain(|i| i != item);

        while self.items.len() >= self.limit {
            self.items.pop_back();
        }

        self.items.push_front(item.into());
    }

    pub fn next(&mut self) -> Option<&str> {
        match self.pos {
            Pos::Last => {
                if !self.items.is_empty() {
                    self.pos = Pos::Index(self.items.len() - 1);
                }
            }
            Pos::Index(n) => {
                self.pos = if n > 0 { Pos::Index(n - 1) } else { Pos::First };
            }
            _ => {}
        }

        self.get()
    }

    pub fn prev(&mut self) -> Option<&str> {
        match self.pos {
            Pos::First => {
                if !self.items.is_empty() {
                    self.pos = Pos::Index(0);
                }
            }
            Pos::Index(n) => {
                let pos = n + 1;
                self.pos = if pos < self.items.len() {
                    Pos::Index(pos)
                } else {
                    Pos::Last
                };
            }
            _ => {}
        }

        self.get()
    }
}

pub(crate) struct Prompt {
    pub message: String,

    input: String,
    cursor: usize,
    completion: Completion,

    /// Called when prompt is confirmed
    pub on_confirm: Option<PAction>,

    /// Called when prompt is aborted
    pub on_abort: Option<PAction>,

    /// Called when input is modified
    pub on_input: Option<PAction>,
    pub keymap: Keymap,

    pub history: History,
}

impl Prompt {
    pub fn new(message: &str) -> Prompt {
        Prompt {
            message: String::from(message),
            input: String::new(),
            cursor: 0,
            completion: Completion::new(false),
            on_confirm: None,
            on_abort: None,
            on_input: None,
            keymap: Keymap::prompt(),
            history: History::new(100),
        }
    }

    pub fn set(&mut self, new: SetPrompt) {
        let SetPrompt {
            message,
            on_confirm,
            on_abort,
            on_input,
            keymap,
        } = new;

        self.message = message;
        self.on_confirm = on_confirm;
        self.on_abort = on_abort;
        self.on_input = on_input;

        if let Some(kmap) = keymap {
            self.keymap = kmap;
        }

        self.history.reset();
        self.input = String::new();
        self.cursor = 0;
    }

    pub fn must_complete(mut self) -> Self {
        self.completion.must_complete = true;
        self
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
        self.completion.match_options(&self.input);
    }

    pub fn next_completion(&mut self) {
        self.completion.select_next();
    }

    pub fn prev_completion(&mut self) {
        self.completion.select_prev();
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
            .field("completions", &self.completion)
            .finish_non_exhaustive()
    }
}
