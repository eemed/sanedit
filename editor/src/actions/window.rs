use crate::{editor::Editor, server::ClientId};

pub(crate) fn scroll_down(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.scroll_down(buf);
}

pub(crate) fn scroll_down_n(editor: &mut Editor, id: ClientId, n: usize) {
    for _ in 0..n {
        scroll_down(editor, id);
    }
}

pub(crate) fn scroll_up(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.scroll_up(buf);
}

pub(crate) fn scroll_up_n(editor: &mut Editor, id: ClientId, n: usize) {
    for _ in 0..n {
        scroll_up(editor, id);
    }
}
