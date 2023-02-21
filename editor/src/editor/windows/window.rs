mod completion;
mod cursors;
mod message;
mod mode;
mod options;
mod prompt;
mod view;

use std::{mem, path::Path};

use sanedit_buffer::piece_tree::prev_grapheme_boundary;
use sanedit_messages::redraw::Size;

use crate::{
    common::{char::DisplayOptions, file::FileMetadata},
    editor::{
        buffers::{Buffer, BufferId},
        options::EditorOptions,
    },
};

pub(crate) use self::{
    cursors::{Cursor, Cursors},
    message::{Message, Severity},
    mode::Mode,
    options::WindowOptions,
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
    mode: Mode,
    pub prompt: Prompt,
    pub options: WindowOptions,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: None,
            cursors: Cursors::default(),
            prompt: Prompt::default(),
            options: WindowOptions::default(),
            mode: Mode::Normal,
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

    pub fn primary_cursor(&self) -> &Cursor {
        self.cursors.primary()
    }

    pub fn primary_cursor_mut(&mut self) -> &mut Cursor {
        self.view.invalidate();
        self.cursors.primary_mut()
    }

    pub fn scroll_down_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );

        let mut view = mem::take(&mut self.view);
        for _ in 0..n {
            view.scroll_down(self, buf);
        }
        self.view = view;

        let primary = self.cursors.primary_mut();
        let range = self.view.range();
        if primary.pos() < range.start {
            primary.goto(range.start);
        }

        log::info!("View down range: {range:?}, {}", primary.pos());
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        let mut view = mem::take(&mut self.view);
        for _ in 0..n {
            view.scroll_up(self, buf);
        }
        self.view = view;

        let primary = self.cursors.primary_mut();
        let range = self.view.range();
        if primary.pos() >= range.end {
            let prev = prev_grapheme_boundary(&buf.slice(..), range.end);
            primary.goto(prev);
        }

        log::info!("View up range: {range:?}, {}", primary.pos());
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
        let mut view = mem::take(&mut self.view);
        view.view_to(cursor, self, buf);
        self.view = view;
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buf
    }

    pub fn view(&self) -> &View {
        &self.view
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

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn open_prompt(&mut self, prompt: Prompt) {
        self.prompt = prompt;
        self.mode = Mode::Prompt;
    }

    pub fn close_prompt(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn cursors(&self) -> &Cursors {
        &self.cursors
    }

    pub fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    pub fn draw_view(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        let mut view = mem::take(&mut self.view);
        view.draw(self, buf);
        self.view = view;
    }
}
