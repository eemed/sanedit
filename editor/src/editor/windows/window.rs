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

pub(crate) use view::Cell;

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
    pub fn new(buf: BufferId) -> Window {
        Window {
            buf,
            view: View::default(),
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

    pub fn redraw(&mut self, buf: &Buffer) {}
}
