use crate::editor::{
    windows::{Focus, Mode},
    Editor,
};

use sanedit_messages::{key::Key, redraw::PopupKind};
use sanedit_server::ClientId;

use super::ActionResult;

#[action("Popup: Close")]
fn close(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    // Dont close signature help
    if win.mode == Mode::Insert {
        if let Some(popup) = win.popup() {
            let is_enter_insert = win.focus == Focus::Window
                && win
                    .keys()
                    .last()
                    .map(|key| key.key() == &Key::Enter)
                    .unwrap_or(false);
            if popup.kind == PopupKind::SignatureHelp && !is_enter_insert {
                return ActionResult::Ok;
            }
        }
    }

    win.clear_popup();
    ActionResult::Ok
}
