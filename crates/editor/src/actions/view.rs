use crate::editor::Editor;

use sanedit_server::ClientId;

#[action("Scroll down")]
fn scroll_down(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.scroll_down_n(buf, 1);
}

#[action("Scroll up")]
fn scroll_up(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.scroll_up_n(buf, 1);
}
