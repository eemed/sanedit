mod completion;
mod cursors;
mod message;
mod mode;
mod options;
mod prompt;
mod view;

use std::mem;

use sanedit_messages::redraw::{self, Redraw, Size};

use crate::editor::buffers::buffer::Buffer;

pub(crate) use self::{
    cursors::{Cursor, Cursors},
    message::{Message, Severity},
    mode::Mode,
    options::WindowOptions,
    prompt::Prompt,
    prompt::PromptAction,
    view::View,
};

use super::BufferId;

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Message,
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
            message: Message::default(),
            cursors: Cursors::default(),
            prompt: Prompt::default(),
            options: WindowOptions::default(),
            mode: Mode::Normal,
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

    fn redraw_view(&mut self, buf: &Buffer) {
        debug_assert!(buf.id == self.buf, "Provided a wrong buffer to window");
        let mut view = mem::take(&mut self.view);
        view.redraw(buf, self);
        self.view = view;
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

    pub fn redraw(&mut self, buf: &Buffer) -> Vec<Redraw> {
        let mut redraw = vec![];
        match self.mode {
            Mode::Normal => {
                self.redraw_view(buf);
                redraw.push(Redraw::Window(redraw::Window::from(&self.view)))
            }
            Mode::Prompt => {}
        }

        redraw
    }

    pub fn cursors(&self) -> &Cursors {
        &self.cursors
    }
}
