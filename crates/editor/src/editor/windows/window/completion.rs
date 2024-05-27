use std::rc::Rc;

use sanedit_messages::redraw::Point;
use sanedit_utils::sorted_vec::SortedVec;

use crate::{
    actions::jobs::{MatchedOptions, MatcherMessage},
    common::cursors::non_whitespace_before_cursor,
    editor::{
        keymap::{DefaultKeyMappings, KeyMappings, Keymap},
        windows::Focus,
        Editor,
    },
    server::ClientId,
};

use super::{selector::Selector, SelectorOption};

pub(crate) type CompletionAction = Rc<dyn Fn(&mut Editor, ClientId, &str)>;

pub(crate) struct Completion {
    pub(crate) point: Point,
    pub(crate) keymap: Keymap,
    pub(crate) selector: Selector,

    /// Called when input is modified.
    pub(crate) on_input: Option<CompletionAction>,
}

impl Completion {
    pub fn new() -> Completion {
        Completion::default()
    }

    pub fn select_next(&mut self) {
        self.selector.select_next()
    }

    pub fn select_prev(&mut self) {
        self.selector.select_prev()
    }

    pub fn provide_options(&mut self, options: SortedVec<SelectorOption>) {
        self.selector.provide_options(options)
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selector.selected_pos()
    }

    pub fn clear_options(&mut self) {
        self.selector = Selector::new();
    }

    pub fn options_window(&self, count: usize, offset: usize) -> Vec<&SelectorOption> {
        self.selector.matches_window(count, offset)
    }

    pub fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
        use MatcherMessage::*;

        // let draw = editor.draw_state(id);
        // draw.no_redraw_window();
        //
        match msg {
            Init(sender) => {
                let word = non_whitespace_before_cursor(editor, id).unwrap_or(String::from(""));
                let _ = sender.blocking_send(word);

                let (win, buf) = editor.win_buf_mut(id);
                win.completion.on_input = Some(Rc::new(move |editor, id, input| {
                    let _ = sender.blocking_send(input.to_string());
                }));
                win.completion.clear_options();
            }
            Progress(opts) => {
                let (win, buf) = editor.win_buf_mut(id);
                match opts {
                    MatchedOptions::Done => {
                        if win.completion.selector.options.is_empty() {
                            win.focus = Focus::Window;
                            win.info_msg("No completion items");
                        }
                    }
                    MatchedOptions::ClearAll => win.completion.clear_options(),
                    MatchedOptions::Options(opts) => {
                        // TODO add descriptions
                        let opts: Vec<SelectorOption> =
                            opts.into_iter().map(SelectorOption::from).collect();
                        let (win, _buf) = editor.win_buf_mut(id);
                        win.completion.provide_options(opts.into());
                    }
                }
            }
        }
    }
}

impl Default for Completion {
    fn default() -> Self {
        Completion {
            point: Point::default(),
            keymap: DefaultKeyMappings::completion(),
            selector: Selector::default(),
            on_input: None,
        }
    }
}

impl std::fmt::Debug for Completion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Completion")
            .field("point", &self.point)
            .field("keymap", &self.keymap)
            .field("selector", &self.selector)
            .finish_non_exhaustive()
    }
}
