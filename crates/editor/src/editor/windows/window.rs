mod completion;
mod cursors;
mod layer;
mod message;
mod options;
mod prompt;
mod search;
mod view;

use std::ops::Range;

use sanedit_buffer::piece_tree::prev_grapheme_boundary;
use sanedit_messages::redraw::Size;

use crate::{
    common::char::DisplayOptions,
    editor::{
        buffers::{Buffer, BufferId},
        keymap::Keymap,
    },
};

pub(crate) use self::{
    cursors::{Cursor, Cursors},
    layer::Layer,
    message::{Message, Severity},
    options::Options,
    prompt::Prompt,
    prompt::PromptAction,
    view::{Cell, View},
};

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Option<Message>,
    cursors: Cursors,

    keymap: Keymap,
    layers: Vec<Layer>,
    pub options: Options,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: None,
            cursors: Cursors::default(),
            keymap: Keymap::default_normal(),
            // prompt: Prompt::default(),
            options: Options::default(),
            layers: Vec::new(),
        }
    }

    pub fn display_options(&self) -> &DisplayOptions {
        &self.view.options
    }

    pub fn open_buffer(&mut self, bid: BufferId) -> BufferId {
        let old = self.buf;
        let width = self.view.width();
        let height = self.view.height();
        *self = Window::new(bid, width, height);
        old
    }

    pub fn info_msg(&mut self, message: String) {
        self.message = Some(Message {
            severity: Severity::Info,
            message,
        });
    }

    pub fn warn_msg(&mut self, message: String) {
        self.message = Some(Message {
            severity: Severity::Warn,
            message,
        });
    }

    pub fn error_msg(&mut self, message: String) {
        self.message = Some(Message {
            severity: Severity::Error,
            message,
        });
    }

    pub fn clear_msg(&mut self) {
        self.message = None;
    }

    pub fn primary_cursor(&self) -> &Cursor {
        self.cursors.primary()
    }

    pub fn primary_cursor_mut(&mut self) -> &mut Cursor {
        self.cursors.primary_mut()
    }

    pub fn scroll_down_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );

        self.view.scroll_down_n(buf, n);
        self.view.redraw(buf);

        let primary = self.cursors.primary_mut();
        let Range { start, end } = self.view.range();
        if primary.pos() < start {
            primary.goto(start);
        }

        log::info!("View down range: {start}..{end}, {}", primary.pos());
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        self.view.scroll_up_n(buf, n);
        self.view.redraw(buf);

        let primary = self.cursors.primary_mut();
        let Range { start, end } = self.view.range();
        if primary.pos() >= end && end != buf.len() {
            let prev = prev_grapheme_boundary(&buf.slice(..), end);
            primary.goto(prev);
        }

        log::info!("View up range: {start}..{end}, {}", primary.pos());
    }

    /// sets window offset so that primary cursor is visible in the drawn view.
    pub fn view_to_cursor(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        let cursor = self.primary_cursor().pos();
        self.view.view_to(cursor, buf);
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buf
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn set_offset(&mut self, offset: usize, buf: &Buffer) {
        self.view.set_offset(offset);
        self.view.redraw(buf);
    }

    pub fn resize(&mut self, size: Size, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        self.view.resize(size);
        self.view_to_cursor(buf);
    }

    pub fn cursors(&self) -> &Cursors {
        &self.cursors
    }

    pub fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    pub fn redraw_view(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        let primary_pos = self.cursors.primary().pos();
        self.view.redraw(buf);
        self.view.view_to(primary_pos, buf);
    }

    pub fn keymap(&self) -> &Keymap {
        for layer in &self.layers {
            if let Some(kmap) = layer.keymap() {
                return kmap;
            }
        }

        &self.keymap
    }

    // If there was no binding for a key, it will be converted to a &str and
    // handled here
    pub fn handle_insert(&mut self, buf: &mut Buffer, text: &str) {
        for layer in &mut self.layers {
            if layer.handle_insert(text) {
                return;
            }
        }

        self.insert_at_cursor(buf, text);
    }

    fn insert_at_cursor(&mut self, buf: &mut Buffer, text: &str) {
        let cursor = self.primary_cursor_mut();
        let cursor_pos = cursor.pos();
        buf.insert(cursor_pos, text);
        cursor.goto(cursor_pos + text.len());
    }

    pub fn layers(&self) -> &[Layer] {
        self.layers.as_slice()
    }

    pub fn prompt(&mut self) -> Option<&mut Prompt> {
        if let Some(Layer::Prompt(ref mut p)) = self.layers.iter_mut().last() {
            Some(p)
        } else {
            None
        }
    }

    pub fn open_prompt(&mut self, prompt: Prompt) {
        self.layers.push(Layer::Prompt(prompt));
    }

    pub fn close_prompt(&mut self) -> Option<Prompt> {
        let is_prompt = matches!(self.layers.iter().last(), Some(Layer::Prompt(..)));
        if let Layer::Prompt(prompt) = self.layers.pop()? {
            Some(prompt)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn to_str(row: &Vec<Cell>) -> String {
        let mut string = String::new();
        for cell in row {
            if let Some(ch) = cell.char() {
                string.push_str(ch.grapheme());
            }
        }
        string
    }

    fn view_lines(win: &Window) -> Vec<String> {
        win.view().cells().iter().map(to_str).collect()
    }

    fn wrapped_line_view() -> (Window, Buffer) {
        let mut buf = Buffer::new();
        buf.insert(
            0,
            "this is a long line that will not fit\nthis is another long line that will not fit into the view\nthis is the third line that is longer than the view",
        );
        let mut win = Window::new(buf.id, 10, 3);
        win.redraw_view(&buf);
        (win, buf)
    }

    fn ten_line() -> (Window, Buffer) {
        let mut buf = Buffer::new();
        buf.insert(
            0,
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten",
        );
        let mut win = Window::new(buf.id, 80, 3);
        win.redraw_view(&buf);
        (win, buf)
    }

    #[test]
    fn scroll_up() {
        let (mut win, buf) = ten_line();
        win.set_offset(14, &buf);
        assert_eq!(vec!["four\n", "five\n", "six\n"], view_lines(&win));
        win.scroll_up_n(&buf, 2);
        assert_eq!(vec!["two\n", "three\n", "four\n"], view_lines(&win));
    }

    #[test]
    fn scroll_up_wrapped() {
        let (mut win, buf) = wrapped_line_view();
        win.set_offset(52, &buf);
        assert_eq!(
            vec!["r long lin", "e that wil", "l not fit "],
            view_lines(&win)
        );

        win.scroll_up_n(&buf, 1);

        assert_eq!(
            vec!["s is anoth", "er long li", "ne that wi"],
            view_lines(&win)
        );

        win.scroll_up_n(&buf, 1);
        assert_eq!(
            vec!["this is an", "other long", " line that"],
            view_lines(&win)
        );
    }

    #[test]
    fn scroll_down() {
        let (mut win, buf) = ten_line();
        win.scroll_down_n(&buf, 2);
        assert_eq!(vec!["three\n", "four\n", "five\n"], view_lines(&win));
    }

    #[test]
    fn scroll_down_wrapped() {
        let (mut win, buf) = wrapped_line_view();
        win.scroll_down_n(&buf, 2);
        assert_eq!(
            vec!["that will ", "not fit\n", "this is an"],
            view_lines(&win)
        );
    }

    #[test]
    fn view_to_after() {
        // let (mut win, buf) = wrapped_line_view();
        // assert_eq!(vec!["", "", ""], view_lines(&win));
    }

    #[test]
    fn view_to_after_small() {}

    #[test]
    fn view_to_before() {}

    #[test]
    fn view_to_before_small() {}
}
