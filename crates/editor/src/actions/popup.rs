use crate::editor::Editor;

use sanedit_server::ClientId;

#[action("Popup: Close")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_popup();
}
