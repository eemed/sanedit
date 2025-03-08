use crate::editor::Editor;

use sanedit_server::ClientId;

use super::ActionResult;

#[action("Popup: Close")]
fn close(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_popup();
    ActionResult::Ok
}
