mod history;

use std::{mem, num::NonZeroUsize, rc::Rc};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    editor::{
        keymap::{DefaultKeyMappings, KeyMappings, Keymap},
        Editor,
    },
    server::{ClientId, JobId},
};

use self::history::History;

use super::{
    selector::{Options, Selector},
    SelectorOption,
};

pub(crate) struct PromptBuilder {
    message: Option<String>,
    on_confirm: Option<PromptAction>,
    on_input: Option<PromptAction>,
    on_abort: Option<PromptAction>,
    keymap: Option<Keymap>,
    history_size: NonZeroUsize,
    background_job: Option<JobId>,
}

impl Default for PromptBuilder {
    fn default() -> Self {
        PromptBuilder {
            message: None,
            on_confirm: None,
            on_input: None,
            on_abort: None,
            keymap: None,
            history_size: NonZeroUsize::new(100).unwrap(),
            background_job: None,
        }
    }
}

impl PromptBuilder {
    pub fn prompt(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());
        self
    }

    pub fn on_input<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.on_input = Some(Rc::new(fun));
        self
    }

    pub fn on_abort<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.on_abort = Some(Rc::new(fun));
        self
    }

    pub fn on_confirm<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.on_confirm = Some(Rc::new(fun));
        self
    }

    pub fn background_job(mut self, id: JobId) -> Self {
        self.background_job = id.into();
        self
    }

    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = Some(keymap);
        self
    }

    pub fn history_size(mut self, size: NonZeroUsize) -> Self {
        self.history_size = size;
        self
    }

    pub fn build(mut self) -> Prompt {
        let PromptBuilder {
            message,
            on_confirm,
            on_input,
            on_abort,
            keymap,
            history_size,
            background_job,
        } = self;
        Prompt {
            message: message.unwrap_or(String::new()),
            input: String::new(),
            cursor: 0,
            selector: Selector::default(),
            on_confirm,
            on_abort,
            on_input,
            keymap: keymap.unwrap_or(DefaultKeyMappings::prompt()),
            history: History::new(history_size.get()),
            background_job,
        }
    }
}

/// Prompt action, similar to a normal `ActionFunction` but also takes the
/// prompt input as a additional parameter
pub(crate) type PromptAction = Rc<dyn Fn(&mut Editor, ClientId, &str)>;

pub(crate) struct Prompt {
    message: String,

    input: String,
    cursor: usize,
    selector: Selector,

    /// Called when prompt is confirmed
    on_confirm: Option<PromptAction>,

    /// Called when prompt is aborted
    on_abort: Option<PromptAction>,

    /// Called when input is modified
    on_input: Option<PromptAction>,

    background_job: Option<JobId>,
    pub keymap: Keymap,

    history: History,
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
            keymap: DefaultKeyMappings::prompt(),
            history: History::new(100),
            background_job: None,
        }
    }

    pub fn builder() -> PromptBuilder {
        PromptBuilder::default()
    }

    pub fn set_on_input<F>(&mut self, fun: F)
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        self.on_input = Some(Rc::new(fun));
    }

    pub fn on_input(&self) -> Option<PromptAction> {
        self.on_input.clone()
    }

    pub fn on_confirm(&self) -> Option<PromptAction> {
        self.on_confirm.clone()
    }

    pub fn on_abort(&self) -> Option<PromptAction> {
        self.on_abort.clone()
    }

    pub fn save_to_history(&mut self) {
        let input = self.input_or_selected();
        self.history.push(&input);
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn background_job(&self) -> Option<JobId> {
        self.background_job
    }

    pub fn clear_options(&mut self) {
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

    pub fn provide_options(&mut self, opts: Options) {
        self.selector.provide_options(opts);
    }

    pub fn options_window(&self, count: usize, offset: usize) -> Vec<&SelectorOption> {
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
