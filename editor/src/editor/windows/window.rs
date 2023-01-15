mod cursors;
mod message;
mod view;

use self::{
    cursors::{Cursor, Cursors},
    message::{Message, Severity},
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
}

impl Window {
    pub fn new(buf: BufferId) -> Window {
        Window {
            buf,
            view: View::default(),
            message: Message::default(),
            cursors: Cursors::default(),
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

    pub fn redraw(&mut self) {
    }
}
