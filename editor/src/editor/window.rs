mod cursor;
mod message;
mod view;

use self::{
    message::{Message, Severity},
    view::View, cursor::Cursor,
};

use super::BufferId;

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Message,
    // All cursor in the current window.
    // Cursror at index 0 is the primary cursor
    cursors: Vec<Cursor>,
}

impl Window {
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
}
