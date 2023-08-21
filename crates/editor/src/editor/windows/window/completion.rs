mod selector;

use sanedit_messages::redraw::Point;

use crate::editor::keymap::Keymap;

pub(crate) use selector::Selector;

#[derive(Debug)]
pub(crate) struct Completion {
    pub(crate) point: Point,
    pub(crate) keymap: Keymap,
    pub(crate) selector: Selector,
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

    pub fn provide_options(&mut self, options: Vec<String>) {
        // self.selector.provide_options(options)
    }

    pub fn selected_pos(&self) -> Option<usize> {
        self.selector.selected_pos()
    }

    pub fn matches_window(&self, count: usize, offset: usize) -> Vec<&str> {
        self.selector.matches_window(count, offset)
    }
}

impl Default for Completion {
    fn default() -> Self {
        Completion {
            point: Point::default(),
            keymap: Keymap::completion(),
            selector: Selector::default(),
        }
    }
}
