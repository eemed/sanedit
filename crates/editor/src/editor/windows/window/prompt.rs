mod history;

use std::rc::Rc;

use sanedit_utils::sorted_vec::SortedVec;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    actions::jobs::{MatchedOptions, MatcherMessage},
    editor::{keymap::KeymapKind, windows::Focus, Editor},
    server::ClientId,
};

pub(crate) use self::history::*;

use super::{selector::Selector, SelectorOption};

pub(crate) struct PromptBuilder {
    message: Option<String>,
    on_confirm: Option<PromptAction>,
    on_input: Option<PromptAction>,
    on_abort: Option<PromptAction>,
    keymap_kind: Option<KeymapKind>,
    simple: bool,
    history_kind: Option<HistoryKind>,
}

impl Default for PromptBuilder {
    fn default() -> Self {
        PromptBuilder {
            message: None,
            on_confirm: None,
            on_input: None,
            on_abort: None,
            keymap_kind: None,
            simple: false,
            history_kind: None,
        }
    }
}

impl PromptBuilder {
    pub fn prompt(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());
        self
    }

    pub fn simple(mut self) -> Self {
        self.simple = true;
        self
    }

    pub fn history(mut self, hist: HistoryKind) -> Self {
        self.history_kind = Some(hist);
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

    pub fn keymap(mut self, keymap: KeymapKind) -> Self {
        self.keymap_kind = Some(keymap);
        self
    }

    pub fn build(self) -> Prompt {
        let PromptBuilder {
            message,
            on_confirm,
            on_input,
            on_abort,
            keymap_kind,
            simple,
            history_kind,
        } = self;
        let mut prompt = Prompt::new(&message.unwrap_or(String::new()));
        prompt.on_confirm = on_confirm;
        prompt.on_abort = on_abort;
        prompt.on_input = on_input;
        prompt.history_kind = history_kind;
        prompt.simple = simple;

        if let Some(kmap) = keymap_kind {
            prompt.keymap_kind = kmap;
        }

        prompt
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

    keymap_kind: KeymapKind,

    history_kind: Option<HistoryKind>,
    history_pos: HistoryPosition,
    simple: bool,
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
            keymap_kind: KeymapKind::Prompt,
            history_kind: None,
            history_pos: HistoryPosition::First,
            simple: false,
        }
    }

    pub fn builder() -> PromptBuilder {
        PromptBuilder::default()
    }

    pub fn keymap(&self) -> KeymapKind {
        self.keymap_kind
    }

    pub fn history(&self) -> Option<HistoryKind> {
        self.history_kind
    }

    pub fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
        use MatcherMessage::*;

        let draw = editor.draw_state(id);
        draw.no_redraw_window();

        let (win, _buf) = editor.win_buf_mut(id);
        match msg {
            Init(sender) => {
                win.prompt.set_on_input(move |editor, id, input| {
                    let _ = sender.blocking_send(input.to_string());
                });
                win.prompt.clear_options();
            }
            Progress(opts) => match opts {
                MatchedOptions::Options { matched, clear_old } => {
                    if clear_old {
                        win.prompt.clear_options();
                    }

                    win.focus = Focus::Prompt;
                    let opts: Vec<SelectorOption> =
                        matched.into_iter().map(SelectorOption::from).collect();
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.prompt.provide_options(opts.into());
                }
                _ => {}
            },
        }
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

    pub fn message(&self) -> &str {
        &self.message
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
            .map(|item| item.value().to_string())
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

    pub fn provide_options(&mut self, opts: SortedVec<SelectorOption>) {
        self.selector.provide_options(opts);
    }

    pub fn options_window(&self, count: usize, offset: usize) -> Vec<&SelectorOption> {
        self.selector.matches_window(count, offset)
    }

    pub fn selected(&self) -> Option<&SelectorOption> {
        self.selector.selected()
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selector.selected_pos()
    }

    pub fn overwrite_input(&mut self, item: &str) {
        self.cursor = item.len();
        self.input = item.into();
    }

    pub fn is_simple(&self) -> bool {
        self.simple
    }

    pub fn history_next(&mut self, hist: &History) {
        use HistoryPosition::*;

        match self.history_pos {
            First => {
                if !hist.is_empty() {
                    self.history_pos = Pos(0);
                }
            }
            Pos(n) => {
                let pos = n + 1;
                self.history_pos = if pos < hist.len() { Pos(pos) } else { Last };
            }
            _ => {}
        }

        let item = hist.get(self.history_pos).unwrap_or("");
        self.overwrite_input(item);
    }

    pub fn history_prev(&mut self, hist: &History) {
        use HistoryPosition::*;

        match self.history_pos {
            Last => {
                if !hist.is_empty() {
                    self.history_pos = Pos(hist.len() - 1);
                }
            }
            Pos(n) => {
                self.history_pos = if n > 0 { Pos(n - 1) } else { First };
            }
            _ => {}
        }

        let item = hist.get(self.history_pos).unwrap_or("");
        self.overwrite_input(item);
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
            .field("simple", &self.simple)
            .finish_non_exhaustive()
    }
}
