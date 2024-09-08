use crate::{
    editor::{hooks::Hook, windows::Focus, Editor},
    server::ClientId,
};

use super::hooks;

#[action("Focus window")]
fn focus(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Reload the current window")]
fn reload(editor: &mut Editor, id: ClientId) {
    editor.reload(id);
}

#[action("Clear messages")]
fn clear_messages(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_msg();
}

#[action("Sync windows if a buffer is changed")]
fn sync_windows(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .map(Hook::buffer_id)
        .flatten()
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        let (win, buf) = editor.win_buf_mut(client);
        win.on_buffer_changed(buf);
    }
}

#[action("Goto previous buffer")]
fn goto_prev_buffer(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if win.goto_prev_buffer() {
        let hook = Hook::BufOpened(buf.id);
        hooks::run(editor, id, hook)
    } else {
        win.warn_msg("No previous buffer");
    }
}
