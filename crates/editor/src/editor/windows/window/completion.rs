use std::rc::Rc;

use sanedit_core::Choice;
use sanedit_messages::redraw::Point;
use sanedit_utils::sorted_vec::SortedVec;

use crate::{
    actions::jobs::{MatchedOptions, MatcherMessage},
    editor::{windows::Focus, Editor},
};
use sanedit_server::ClientId;

use super::chooser::Chooser;

pub(crate) type CompletionAction = Rc<dyn Fn(&mut Editor, ClientId, &str)>;

#[derive(Default)]
pub(crate) struct Completion {
    point: Point,

    /// Where the completion was started at
    /// used to provide on input with the next term
    started_at: u64,

    chooser: Chooser,

    /// Called when input is modified.
    pub(crate) on_input: Option<CompletionAction>,
}

impl Completion {
    pub fn new(started_at: u64, point: Point) -> Completion {
        Completion {
            started_at,
            point,
            ..Default::default()
        }
    }

    pub fn started_at(&self) -> u64 {
        self.started_at
    }

    pub fn point(&self) -> &Point {
        &self.point
    }

    pub fn selected(&self) -> Option<&Choice> {
        self.chooser.selected()
    }

    pub fn select_next(&mut self) {
        self.chooser.select_next()
    }

    pub fn select_prev(&mut self) {
        self.chooser.select_prev()
    }

    pub fn add_choices(&mut self, options: SortedVec<Choice>) {
        self.chooser.add(options)
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.chooser.selected_pos()
    }

    pub fn clear_choices(&mut self) {
        self.chooser = Chooser::new();
    }

    pub fn choices_part(&self, count: usize, offset: usize) -> Vec<&Choice> {
        self.chooser.matches_window(count, offset)
    }

    pub fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
        use MatcherMessage::*;

        let draw = editor.draw_state(id);
        draw.no_redraw_window();

        match msg {
            Init(sender) => {
                let (win, buf) = editor.win_buf_mut(id);
                let cursor = win.cursors.primary().pos();
                let start = win.completion.started_at;
                let slice = buf.slice(start..cursor);
                let word = String::from(&slice);
                let _ = sender.blocking_send(word);

                let (win, _buf) = editor.win_buf_mut(id);
                win.completion.on_input = Some(Rc::new(move |_editor, _id, input| {
                    let _ = sender.blocking_send(input.to_string());
                }));
                win.completion.clear_choices();
            }
            Progress(opts) => {
                let (win, _buf) = editor.win_buf_mut(id);
                match opts {
                    MatchedOptions::Done => {
                        if win.completion.chooser.options().is_empty() {
                            win.focus = Focus::Window;
                            win.info_msg("No completion items");
                        }
                    }
                    MatchedOptions::Options { matched, clear_old } => {
                        if clear_old {
                            win.completion.clear_choices();
                        }
                        win.focus = Focus::Completion;
                        let opts: Vec<Choice> = matched.into_iter().map(Choice::from).collect();
                        let (win, _buf) = editor.win_buf_mut(id);
                        win.completion.add_choices(opts.into());
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for Completion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Completion")
            .field("point", &self.point)
            .field("selector", &self.chooser)
            .finish_non_exhaustive()
    }
}
