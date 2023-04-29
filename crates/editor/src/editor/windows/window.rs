mod completion;
mod cursors;
mod focus;
mod message;
mod options;
mod prompt;
mod search;
mod view;
mod locations;

use std::ops::Range;

use sanedit_buffer::piece_tree::prev_grapheme_boundary;
use sanedit_messages::redraw::Size;

use crate::{
    common::{char::DisplayOptions, movement},
    editor::{
        buffers::{Buffer, BufferId},
        keymap::Keymap,
    },
};

pub(crate) use self::{
    cursors::{Cursor, Cursors},
    focus::Focus,
    message::{Message, Severity},
    options::Options,
    prompt::PAction,
    prompt::Prompt,
    prompt::SetPrompt,
    search::Search,
    search::SetSearch,
    view::{Cell, View},
};

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Option<Message>,
    cursors: Cursors,

    keymap: Keymap,
    pub focus: Focus,
    pub search: Search,
    pub prompt: Prompt,
    pub options: Options,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: None,
            cursors: Cursors::default(),
            keymap: Keymap::window(),
            options: Options::default(),
            search: Search::default(),
            prompt: Prompt::default(),
            focus: Focus::Window,
        }
    }

    pub fn focus(&self) -> Focus {
        self.focus
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
        let Range { start, .. } = self.view.range();
        if primary.pos() < start {
            primary.goto(start);
        }
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
        let pos = primary.pos();
        if !self.view.is_visible(pos) {
            let prev = prev_grapheme_boundary(&buf.slice(..), self.view.end());
            primary.goto(prev);
        }
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

    pub fn invalidate_view(&mut self) {
        self.view.invalidate();
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
        match self.focus {
            Focus::Search => &self.search.prompt.keymap,
            Focus::Prompt => &self.prompt.keymap,
            Focus::Window => &self.keymap,
        }
    }

    fn remove_cursor_selection(&mut self, buf: &mut Buffer) -> bool {
        let cursor = self.primary_cursor_mut();
        if let Some(sel) = cursor.take_selection() {
            cursor.goto(sel.start);
            buf.remove(sel);
            self.invalidate_view();
            true
        } else {
            false
        }
    }

    pub fn insert_at_cursor(&mut self, buf: &mut Buffer, text: &str) {
        self.remove_cursor_selection(buf);
        let cursor = self.primary_cursor_mut();
        let cursor_pos = cursor.pos();
        buf.insert(cursor_pos, text);
        cursor.goto(cursor_pos + text.len());
        self.invalidate_view();
    }

    pub fn remove_grapheme_after_cursor(&mut self, buf: &mut Buffer) {
        if self.remove_cursor_selection(buf) {
            return;
        }
        let cursor = self.primary_cursor_mut();
        let pos = movement::next_grapheme_boundary(&buf.slice(..), cursor.pos());
        buf.remove(cursor.pos()..pos);
        self.invalidate_view();
    }

    pub fn remove_grapheme_before_cursor(&mut self, buf: &mut Buffer) {
        if self.remove_cursor_selection(buf) {
            return;
        }
        let cursor = self.primary_cursor_mut();
        let pos = movement::prev_grapheme_boundary(&buf.slice(..), cursor.pos());
        buf.remove(pos..cursor.pos());
        cursor.goto(pos);
        self.invalidate_view();
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
        win.view.set_offset(14);
        win.view.redraw(&buf);
        assert_eq!(vec!["four\n", "five\n", "six\n"], view_lines(&win));
        win.scroll_up_n(&buf, 2);
        assert_eq!(vec!["two\n", "three\n", "four\n"], view_lines(&win));
    }

    #[test]
    fn scroll_up_wrapped() {
        let (mut win, buf) = wrapped_line_view();
        win.view.set_offset(52);
        win.view.redraw(&buf);
        assert_eq!(
            vec!["r long lin", "e that wil", "l not fit "],
            view_lines(&win)
        );

        win.scroll_up_n(&buf, 1);
        assert_eq!(
            vec!["this is an", "other long", " line that"],
            view_lines(&win)
        );

        win.scroll_up_n(&buf, 1);
        assert_eq!(
            vec!["a long lin", "e that wil", "l not fit\n"],
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
