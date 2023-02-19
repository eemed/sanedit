use sanedit_messages::redraw::Statusline;

use crate::editor::{buffers::Buffer, windows::Window};

pub(crate) fn draw_statusline(win: &Window, buf: &Buffer) -> Statusline {
    let line = match win.message() {
        Some(msg) => format!("{}", msg.message),
        None => format!("{}", buf.name()),
    };

    Statusline::new(line.as_str())
}
