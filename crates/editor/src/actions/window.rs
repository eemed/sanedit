use crate::editor::{hooks::Hook, windows::Focus, Editor};

use sanedit_server::ClientId;

use super::{cursors, hooks};

#[action("Focus window")]
fn focus_window(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Reload the current window")]
fn reload_window(editor: &mut Editor, id: ClientId) {
    editor.reload(id);
}

#[action("Clear messages")]
fn clear_messages(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_msg();
}

#[action("Sync windows if a buffer is changed")]
fn sync_windows(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
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

#[action("Progressively close stuff on the screen")]
fn prog_cancel(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    if win.search.hl_last || win.popup().is_some() {
        // Clear search matches
        win.search.hl_last = false;
        win.search.hl_matches.clear();

        // Close popups
        win.clear_popup();
        return;
    }

    cursors::keep_only_primary.execute(editor, id);
}

#[action("Persist keys pressed")]
fn persist(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.key_persist = win.keys().len();
}

#[action("Clear persisted keys")]
fn clear_persist(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.key_persist = 0;
}
