use crate::editor::{windows::Mode, Editor};

use sanedit_server::ClientId;

use super::ActionResult;

#[action("Popup: Close")]
fn close(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_popup();
    ActionResult::Ok
}

#[action("Popup: Close popup")]
fn close_popup(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    // Dont close signature help
    if win.mode == Mode::Insert {
        return ActionResult::Ok;
    }

    win.clear_popup();
    ActionResult::Ok
}
