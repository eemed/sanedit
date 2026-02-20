use crate::editor::Editor;

use sanedit_server::ClientId;

use super::ActionResult;

#[action("View: Scroll down")]
fn scroll_down(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    win.scroll_down_n(buf, 1);

    ActionResult::Ok
}

#[action("View: Scroll up")]
fn scroll_up(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    win.scroll_up_n(buf, 1);

    ActionResult::Ok
}
