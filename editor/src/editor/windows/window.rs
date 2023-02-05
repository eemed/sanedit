mod completion;
mod cursors;
mod message;
mod options;
mod prompt;
mod view;

use sanedit_messages::redraw::Size;

use crate::editor::buffers::buffer::Buffer;

use self::{
    cursors::{Cursor, Cursors},
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
    prompt: Option<Prompt>,

    options: WindowOptions,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: Message::default(),
            cursors: Cursors::default(),
            prompt: None,
            options: WindowOptions::default(),
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

    pub fn prompt_mut(&mut self) -> Option<&mut Prompt> {
        self.prompt.as_mut()
    }

    pub fn prompt_take(&mut self) -> Option<Prompt> {
        self.prompt.take()
    }
}
