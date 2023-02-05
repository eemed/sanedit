mod completion;
mod cursors;
mod input;
mod message;
mod options;
mod prompt;
mod view;

use std::mem;

use sanedit_messages::redraw::Size;

use crate::editor::buffers::buffer::Buffer;

pub(crate) use self::{
    cursors::{Cursor, Cursors},
    input::InputMode,
    message::{Message, Severity},
    options::WindowOptions,
    prompt::Prompt,
    view::View,
};

use super::BufferId;

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Message,
    cursors: Cursors,
    prompt: Prompt,
    input: InputMode,
    options: WindowOptions,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: Message::default(),
            cursors: Cursors::default(),
            prompt: Prompt::default(),
            options: WindowOptions::default(),
            input: InputMode::Normal,
        }
    }

    pub fn info_msg(&mut self, message: String) {
        self.message = Message {
            severity: Severity::Info,
            message,
        };
    }

    pub fn warn_msg(&mut self, message: String) {
        // TODO better way to save these bytes?
        self.message = Message {
            severity: Severity::Warn,
            message,
        };
    }

    pub fn error_msg(&mut self, message: String) {
        self.message = Message {
            severity: Severity::Error,
            message,
        };
    }

    pub fn primary_cursor(&self) -> &Cursor {
        self.cursors.primary()
    }

    pub fn primary_cursor_mut(&mut self) -> &mut Cursor {
        self.cursors.primary_mut()
    }

    pub fn redraw(&mut self, buf: &Buffer) {
        debug_assert!(buf.id == self.buf, "Provided a wrong buffer to window");
        self.view.redraw(&buf, &self.cursors, &self.options.display)
    }

    pub fn scroll_down(&mut self, buf: &Buffer) {
        debug_assert!(buf.id == self.buf, "Provided a wrong buffer to window");
        todo!()
    }

    pub fn scroll_up(&mut self, buf: &Buffer) {
        debug_assert!(buf.id == self.buf, "Provided a wrong buffer to window");
        todo!()
    }

    ///  sets window offset so that primary cursor is visible in the drawn view.
    pub fn view_to_cursor(&mut self) {
        todo!()
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buf
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn resize(&mut self, size: Size) {
        self.view.resize(size);
    }

    pub fn needs_redraw(&self) -> bool {
        self.view.needs_redraw()
    }

    pub fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    pub fn prompt_mut(&mut self) -> &mut Prompt {
        &mut self.prompt
    }

    pub fn take_prompt(&mut self) -> Prompt {
        mem::replace(&mut self.prompt, Prompt::default())
    }

    pub fn input_mode(&self) -> InputMode {
        self.input
    }

    pub fn open_prompt(&mut self, prompt: Prompt) {
        self.prompt = prompt;
        self.input = InputMode::Prompt;
    }

    pub fn close_prompt(&mut self) {
        self.input = InputMode::Normal;
    }
}
