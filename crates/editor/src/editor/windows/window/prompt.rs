mod history;

use std::{path::PathBuf, rc::Rc};

use sanedit_buffer::PieceTree;
use sanedit_utils::{either::Either, sorted_vec::SortedVec};

use crate::{
    actions::{
        jobs::MatcherMessage, prompt::get_directory_searcher_term, window::focus, ActionResult,
    },
    common::{Choice, ScoredChoice},
    editor::{snippets::Snippet, windows::Focus, Editor},
};
use sanedit_server::ClientId;

pub(crate) use self::history::*;

use super::chooser::Chooser;

#[derive(Default)]
pub(crate) struct PromptBuilder {
    message: Option<String>,
    on_confirm: Option<PromptOnConfirm>,
    on_input: Option<PromptOnInput>,
    on_abort: Option<PromptOnAbort>,
    kind: PromptKind,
    history_kind: Option<HistoryKind>,
    input: String,
}

impl PromptBuilder {
    pub fn prompt(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());
        self
    }

    pub fn simple(mut self) -> Self {
        self.kind = PromptKind::Simple;
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
        F: Fn(&mut Editor, ClientId, PromptOutput) + 'static,
    {
        self.on_abort = Some(Rc::new(fun));
        self
    }

    pub fn on_confirm<F>(mut self, fun: F) -> Self
    where
        F: Fn(&mut Editor, ClientId, PromptOutput) -> ActionResult + 'static,
    {
        self.on_confirm = Some(Box::new(fun));
        self
    }

    pub fn build(self) -> Prompt {
        let PromptBuilder {
            message,
            on_confirm,
            on_input,
            on_abort,
            kind,
            history_kind,
            input,
        } = self;
        let mut prompt = Prompt::new(&message.unwrap_or(String::new()));
        prompt.on_confirm = on_confirm;
        prompt.on_abort = on_abort;
        prompt.on_input = on_input;
        prompt.history_kind = history_kind;
        prompt.kind = kind;
        prompt.cursor = input.len();
        prompt.input = input;

        prompt
    }
}

/// Prompt action, similar to a normal `ActionFunction` but also takes the
/// prompt input as a additional parameter
pub(crate) type PromptOnConfirm =
    Box<dyn FnOnce(&mut Editor, ClientId, PromptOutput) -> ActionResult>;
pub(crate) type PromptOnAbort = Rc<dyn Fn(&mut Editor, ClientId, PromptOutput)>;
pub(crate) type PromptOnInput = Rc<dyn Fn(&mut Editor, ClientId, &str)>;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PromptOutput {
    inner: Either<String, ScoredChoice>,
}

impl PromptOutput {
    pub fn path(&self) -> Option<PathBuf> {
        match &self.inner {
            Either::Left(text) => Some(PathBuf::from(text)),
            Either::Right(choice) => match choice.choice() {
                Choice::Path { path, .. } => Some(path.clone()),
                Choice::Text { text, .. } => Some(PathBuf::from(text)),
                _ => None,
            },
        }
    }

    pub fn path_selection(&self) -> Option<PathBuf> {
        match &self.inner {
            Either::Right(choice) => match choice.choice() {
                Choice::Path { path, .. } => Some(path.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn snippet(&self) -> Option<&Snippet> {
        match &self.inner {
            Either::Right(choice) => match choice.choice() {
                Choice::Snippet { snippet, .. } => Some(snippet),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match &self.inner {
            Either::Left(text) => Some(text),
            Either::Right(choice) => match choice.choice() {
                Choice::Text { text, .. } => Some(text.as_str()),
                _ => None,
            },
        }
    }

    pub fn number(&self) -> Option<usize> {
        match &self.inner {
            Either::Left(text) => text.parse::<usize>().ok(),
            Either::Right(choice) => choice.choice().number(),
        }
    }

    pub fn is_selection(&self) -> bool {
        match self.inner {
            Either::Left(_) => false,
            Either::Right(_) => true,
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub(crate) enum PromptKind {
    #[default]
    Regular,

    Simple,
}

pub(crate) struct Prompt {
    message: String,

    input: String,
    cursor: usize,
    chooser: Chooser,

    /// Called when prompt is confirmed
    on_confirm: Option<PromptOnConfirm>,

    /// Called when prompt is aborted
    on_abort: Option<PromptOnAbort>,

    /// Called when input is modified
    on_input: Option<PromptOnInput>,

    history_kind: Option<HistoryKind>,
    history_pos: HistoryPosition,
    kind: PromptKind,

    /// Id that changes every time input is changed.
    /// used to identify matcher messages that are relevant
    /// as there may be old messages coming in too
    input_id: u64,
    is_options_loading: bool,
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
            history_kind: None,
            history_pos: HistoryPosition::First,
            kind: PromptKind::Regular,
            is_options_loading: false,
            input_id: 0,
        }
    }

    pub fn builder() -> PromptBuilder {
        PromptBuilder::default()
    }

    pub fn history(&self) -> Option<HistoryKind> {
        self.history_kind
    }

    pub fn is_options_loading(&self) -> bool {
        self.is_options_loading
    }

    pub fn matcher_result_handler_directory_selector(
        editor: &mut Editor,
        id: ClientId,
        msg: MatcherMessage,
    ) {
        use MatcherMessage::*;

        let draw = editor.draw_state(id);
        draw.no_redraw_window();

        let (win, _buf) = editor.win_buf_mut(id);
        let is_progress = matches!(msg, Progress { .. });
        match msg {
            Init(sender) => {
                win.prompt.input_id = 0;
                win.prompt.add_on_input(move |editor, id, input| {
                    let path = PathBuf::from(input);
                    let input = path.to_string_lossy();
                    let input = get_directory_searcher_term(&input);
                    let (win, _buf) = editor.win_buf_mut(id);
                    let _ = sender.blocking_send((input.to_string(), win.prompt.input_id));
                });
                win.prompt.clear_choices();
                win.prompt.is_options_loading = true;
            }
            Done {
                results,
                clear_old,
                input_id,
            }
            | Progress {
                results,
                clear_old,
                input_id,
            } => {
                if input_id != win.prompt.input_id {
                    return;
                }
                win.prompt.is_options_loading = is_progress;
                if clear_old {
                    win.prompt.clear_choices();
                }

                let (win, _buf) = editor.win_buf_mut(id);
                results
                    .into_iter()
                    .for_each(|res| win.prompt.add_choices(res));

                focus(editor, id, Focus::Prompt);
            }
        }
    }

    pub fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
        use MatcherMessage::*;

        let draw = editor.draw_state(id);
        draw.no_redraw_window();

        let (win, _buf) = editor.win_buf_mut(id);
        let is_progress = matches!(msg, Progress { .. });
        match msg {
            Init(sender) => {
                win.prompt.add_on_input(move |editor, id, input| {
                    let (win, _buf) = editor.win_buf(id);
                    let _ = sender.blocking_send((input.to_string(), win.prompt.input_id));
                });
                win.prompt.clear_choices();
                win.prompt.is_options_loading = true;
            }
            Done {
                results,
                clear_old,
                input_id,
            }
            | Progress {
                results,
                clear_old,
                input_id,
            } => {
                if input_id != win.prompt.input_id {
                    return;
                }
                win.prompt.is_options_loading = is_progress;
                if clear_old {
                    win.prompt.clear_choices();
                }

                let (win, _buf) = editor.win_buf_mut(id);
                results
                    .into_iter()
                    .for_each(|res| win.prompt.add_choices(res));

                focus(editor, id, Focus::Prompt);
            }
        }
    }

    pub fn open_file_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
        use MatcherMessage::*;

        let draw = editor.draw_state(id);
        draw.no_redraw_window();

        let (win, _buf) = win_buf!(editor, id);
        let is_progress = matches!(msg, Progress { .. });
        match msg {
            Init(sender) => {
                win.prompt.add_on_input(move |editor, id, input| {
                    let (win, _buf) = editor.win_buf_mut(id);
                    let _ = sender.blocking_send((input.to_string(), win.prompt.input_id));
                });
                win.prompt.clear_choices();
                win.prompt.is_options_loading = true;
            }
            Done {
                results,
                clear_old,
                input_id,
            }
            | Progress {
                results,
                clear_old,
                input_id,
            } => {
                if input_id != win.prompt.input_id {
                    return;
                }

                win.prompt.is_options_loading = is_progress;
                if clear_old {
                    win.prompt.clear_choices();
                }

                let no_input = results
                    .get(0)
                    .map(|res| {
                        res.get(0)
                            .map(|choice| choice.matches().is_empty())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                let (win, _buf) = editor.win_buf_mut(id);

                if no_input {
                    // If no input is matched, sort results using LRU
                    for res in results {
                        let mru = &mut editor.caches.files;
                        let max = mru.len();
                        let mut rescored_batch = Vec::with_capacity(res.len());
                        for mut choice in res.into_iter() {
                            let path = match choice.choice() {
                                Choice::Path { path, .. } => path,
                                _ => unreachable!(),
                            };
                            if let Some(score) = mru.position(&path) {
                                // Smallest first so reverse position
                                choice.rescore(max - (score + 1));
                            } else {
                                choice.rescore(choice.score() + max);
                            }
                            rescored_batch.push(choice);
                        }

                        let (win, _buf) = editor.win_buf_mut(id);
                        win.prompt.add_choices(SortedVec::from(rescored_batch))
                    }
                } else {
                    results
                        .into_iter()
                        .for_each(|res| win.prompt.add_choices(res));
                }

                focus(editor, id, Focus::Prompt);
            }
        }
    }

    pub fn add_on_input<F>(&mut self, fun: F)
    where
        F: Fn(&mut Editor, ClientId, &str) + 'static,
    {
        let fun = Rc::new(fun);
        if let Some(on_input) = std::mem::take(&mut self.on_input) {
            // Add both
            self.on_input = Some(Rc::new(move |e, id, input| {
                (on_input)(e, id, input);
                (fun)(e, id, input);
            }));
        } else {
            self.on_input = Some(fun);
        }
    }

    pub fn set_on_confirm<F>(&mut self, fun: F)
    where
        F: FnOnce(&mut Editor, ClientId, PromptOutput) -> ActionResult + 'static,
    {
        self.on_confirm = Some(Box::new(fun));
    }

    pub fn on_input(&self) -> Option<PromptOnInput> {
        self.on_input.clone()
    }

    pub fn on_confirm(&mut self) -> Option<PromptOnConfirm> {
        self.on_confirm.take()
    }

    pub fn on_abort(&self) -> Option<PromptOnAbort> {
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
        if start != end {
            self.input_id += 1;
            self.input.replace_range(start..end, "");
        }
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

    pub fn add_choices(&mut self, opts: SortedVec<ScoredChoice>) {
        self.chooser.add(opts);
    }

    #[allow(dead_code)]
    pub fn total_choices(&self) -> usize {
        self.chooser.len()
    }

    pub fn options_window(&self, count: usize, offset: usize) -> Vec<&ScoredChoice> {
        self.chooser.matches_window(count, offset)
    }

    pub fn selected(&self) -> Option<&ScoredChoice> {
        self.chooser.selected()
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.chooser.selected_pos()
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn input_or_selected(&self) -> PromptOutput {
        let inner = match self.selected() {
            Some(scored) => Either::Right(scored.clone()),
            None => Either::Left(self.input.clone()),
        };

        PromptOutput { inner }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert_at_cursor(&mut self, string: &str) {
        self.input_id += 1;
        self.input.insert_str(self.cursor, string);
        self.cursor += string.len();
    }

    pub fn overwrite_input(&mut self, item: &str) {
        self.input_id += 1;
        self.cursor = item.len();
        self.input = item.into();
    }

    pub fn is_simple(&self) -> bool {
        self.kind == PromptKind::Simple
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
            .field("completions", &self.chooser)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
