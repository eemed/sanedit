use std::rc::Rc;

use sanedit_messages::redraw::Point;
use sanedit_utils::sorted_vec::SortedVec;

use crate::{
    actions::{jobs::MatcherMessage, window::focus},
    common::ScoredChoice,
    editor::{windows::Focus, Editor},
};
use sanedit_server::ClientId;

use super::chooser::Chooser;

pub(crate) type CompletionAction = Rc<dyn Fn(&mut Editor, ClientId, &str)>;

#[derive(Default)]
pub(crate) struct Completion {
    /// Point at which completion was started
    point: Point,
    /// Point as offset
    point_offset: u64,

    /// Where the completion item starts at
    /// used to provide on input with the next term
    /// This may be before point if completed in middle of word
    item_start: u64,

    chooser: Chooser,

    /// Called when input is modified.
    pub(crate) on_input: Option<CompletionAction>,

    input_id: u64,
}

impl Completion {
    pub fn new(started_at: u64, started_at_cursor: u64, point: Point) -> Completion {
        Completion {
            item_start: started_at,
            point_offset: started_at_cursor,
            point,
            chooser: Chooser::default(),
            on_input: None,
            input_id: 0,
        }
    }

    pub fn point_offset(&self) -> u64 {
        self.point_offset
    }

    pub fn item_start(&self) -> u64 {
        self.item_start
    }

    pub fn point(&self) -> &Point {
        &self.point
    }

    pub fn selected(&self) -> Option<&ScoredChoice> {
        self.chooser.selected()
    }

    pub fn select_next(&mut self) {
        self.chooser.select_next()
    }

    pub fn select_prev(&mut self) {
        self.chooser.select_prev()
    }

    pub fn add_choices(&mut self, options: SortedVec<ScoredChoice>) {
        self.chooser.add(options)
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.chooser.selected_pos()
    }

    pub fn clear_choices(&mut self) {
        self.chooser = Chooser::new();
    }

    pub fn choices_part(&self, count: usize, offset: usize) -> Vec<&ScoredChoice> {
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
                let start = win.completion.item_start;
                let slice = buf.slice(start..cursor);
                let word = String::from(&slice);
                let _ = sender.blocking_send((word, win.completion.input_id));

                let (win, _buf) = editor.win_buf_mut(id);
                win.completion.on_input = Some(Rc::new(move |editor, id, input| {
                    let (win, _buf) = editor.win_buf_mut(id);
                    let _ = sender.blocking_send((input.to_string(), win.completion.input_id));
                }));
                win.completion.clear_choices();
            }
            Progress {
                results,
                clear_old,
                input_id,
            } => {
                let (win, _buf) = editor.win_buf_mut(id);
                if input_id != win.completion.input_id {
                    return;
                }
                if clear_old {
                    win.completion.clear_choices();
                }
                results
                    .into_iter()
                    .for_each(|res| win.completion.add_choices(res));

                if win.focus() != Focus::Completion {
                    focus(editor, id, Focus::Completion);
                }
            }
            Done {
                results,
                clear_old,
                input_id,
            } => {
                let (win, _buf) = editor.win_buf_mut(id);
                if input_id != win.completion.input_id {
                    return;
                }
                if clear_old {
                    win.completion.clear_choices();
                }
                results
                    .into_iter()
                    .for_each(|res| win.completion.add_choices(res));

                if win.focus() != Focus::Completion {
                    focus(editor, id, Focus::Completion);
                }

                let (win, _buf) = editor.win_buf_mut(id);
                if win.completion.chooser.options().is_empty() {
                    win.info_msg("No completion items");
                    focus(editor, id, Focus::Window);
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
