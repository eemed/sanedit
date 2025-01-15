mod history;

use std::rc::Rc;

use sanedit_buffer::PieceTree;
use sanedit_core::Choice;
use sanedit_utils::sorted_vec::SortedVec;

use crate::{
    actions::jobs::{MatchedOptions, MatcherMessage},
    editor::{keymap::KeymapKind, windows::Focus, Editor},
};
use sanedit_server::ClientId;

pub(crate) use self::history::*;

use super::chooser::Chooser;

#[derive(Default)]
pub(crate) struct PromptBuilder {
    message: Option<String>,
    on_confirm: Option<PromptAction>,
    on_input: Option<PromptAction>,
    on_abort: Option<PromptAction>,
    keymap_kind: Option<KeymapKind>,
    simple: bool,
    has_paths: bool,
    history_kind: Option<HistoryKind>,
    input: String,
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

    pub fn has_paths(mut self) -> Self {
        self.has_paths = true;
        self
    }

    pub fn input(mut self, input: &str) -> Self {
        self.input = input.into();
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

    #[allow(dead_code)]
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
            input,
            has_paths: paths,
        } = self;
        let mut prompt = Prompt::new(&message.unwrap_or(String::new()));
        prompt.on_confirm = on_confirm;
        prompt.on_abort = on_abort;
        prompt.on_input = on_input;
        prompt.history_kind = history_kind;
        prompt.simple = simple;
        prompt.cursor = input.len();
        prompt.input = input;
        prompt.has_paths = paths;

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
    chooser: Chooser,

    /// Called when prompt is confirmed
    on_confirm: Option<PromptAction>,

    /// Called when prompt is aborted
    on_abort: Option<PromptAction>,

    /// Called when input is modified
    on_input: Option<PromptAction>,

    keymap_kind: KeymapKind,

    history_kind: Option<HistoryKind>,
    history_pos: HistoryPosition,
    /// show this prompt as a simple prompt
    simple: bool,

    /// If selector contains paths
    has_paths: bool,
}

impl Prompt {
    pub fn new(message: &str) -> Prompt {
        Prompt {
            message: String::from(message),
            input: String::new(),
            cursor: 0,
            chooser: Chooser::new(),
            on_confirm: None,
            on_abort: None,
            on_input: None,
            keymap_kind: KeymapKind::Prompt,
            history_kind: None,
            history_pos: HistoryPosition::First,
            simple: false,
            has_paths: false,
        }
    }

    pub fn builder() -> PromptBuilder {
        PromptBuilder::default()
    }

    pub fn keymap_kind(&self) -> KeymapKind {
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
                win.prompt.set_on_input(move |_editor, _id, input| {
                    let _ = sender.blocking_send(input.to_string());
                });
                win.prompt.clear_choices();
            }
            Progress(opts) => {
                if let MatchedOptions::Options { matched, clear_old } = opts {
                    if clear_old {
                        win.prompt.clear_choices();
                    }

                    win.focus_to(Focus::Prompt);
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.prompt.add_choices(matched.into());
                }
            }
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

    pub fn next_grapheme(&mut self) {
        let pt = PieceTree::from(&self.input);
        let slice = pt.slice(self.cursor as u64..);
        let mut graphemes = slice.graphemes();
        graphemes.next();
        self.cursor = graphemes
            .next()
            .map_or(self.input.len(), |slice| slice.start() as usize);
    }

    pub fn prev_grapheme(&mut self) {
        let pt = PieceTree::from(&self.input);
        let slice = pt.slice(..self.cursor as u64);
        let mut graphemes = slice.graphemes_at(slice.len());
        self.cursor = graphemes.prev().map_or(0, |slice| slice.start() as usize);
    }

    pub fn remove_grapheme_before_cursor(&mut self) {
        let end = self.cursor;
        self.prev_grapheme();
        let start = self.cursor;
        self.input.replace_range(start..end, "");
    }

    pub fn clear_choices(&mut self) {
        self.chooser = Chooser::new();
    }

    pub fn next_completion(&mut self) {
        self.chooser.select_next();
    }

    pub fn prev_completion(&mut self) {
        self.chooser.select_prev();
    }

    pub fn add_choices(&mut self, opts: SortedVec<Choice>) {
        self.chooser.add(opts);
    }

    pub fn options_window(&self, count: usize, offset: usize) -> Vec<&Choice> {
        self.chooser.matches_window(count, offset)
    }

    pub fn selected(&self) -> Option<&Choice> {
        self.chooser.selected()
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.chooser.selected_pos()
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn input_or_selected(&self) -> String {
        self.selected()
            .map(|item| item.to_str_lossy().to_string())
            .unwrap_or(self.input.clone())
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_at_cursor(&mut self, string: &str) {
        self.input.insert_str(self.cursor, string);
        self.cursor += string.len();
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

    pub fn has_paths(&self) -> bool {
        self.has_paths
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
            .field("completions", &self.chooser)
            .field("simple", &self.simple)
            .finish_non_exhaustive()
    }
}
