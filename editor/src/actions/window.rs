use crate::{editor::Editor, server::ClientId};

pub(crate) fn scroll_down(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.scroll_down_n(buf, 1);
}

pub(crate) fn scroll_up(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.scroll_up_n(buf, 1);
}
