use sanedit_messages::redraw::Statusline;

use crate::editor::{buffers::Buffer, windows::Window};

pub(crate) fn draw_statusline(win: &Window, buf: &Buffer) -> Statusline {
    let line = format!("{}", buf.name());
    Statusline::new(line.as_str())
}
