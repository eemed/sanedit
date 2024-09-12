use sanedit_messages::redraw::{Severity, StatusMessage};

use crate::editor::Editor;

use sanedit_server::ClientId;

#[action("Popup a message")]
fn test(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);

    win.popup = Some(StatusMessage {
        severity: Severity::Info,
        message: "Hello world\nand another line".into(),
    });
}

#[action("Close popup message")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.popup = None;
}
