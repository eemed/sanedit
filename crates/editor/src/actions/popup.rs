use crate::editor::{windows::Mode, Editor};

use sanedit_messages::redraw::PopupKind;
use sanedit_server::ClientId;

use super::ActionResult;

#[action("Popup: Close")]
fn close(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    // Dont close signature help
    if win.mode == Mode::Insert {
        if let Some(popup) = win.popup() {
            if popup.kind == PopupKind::SignatureHelp {
                return ActionResult::Ok;
            }
        }
    }

    win.clear_popup();
    ActionResult::Ok
}
