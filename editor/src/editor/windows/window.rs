mod cursors;
mod message;
mod options;
mod view;

use crate::editor::buffers::buffer::Buffer;

use self::{
    cursors::{Cursor, Cursors},
    message::{Message, Severity},
    options::WindowOptions,
    view::View,
};

use super::BufferId;

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Message,
    cursors: Cursors,

    options: WindowOptions,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: Message::default(),
            cursors: Cursors::default(),
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
    }

    pub fn scroll_up(&mut self, buf: &Buffer) {}

    pub fn buffer_id(&self) -> BufferId {
        self.buf
    }

    pub fn view(&self) -> &View {
        &self.view
    }
}
