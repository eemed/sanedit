use crate::{editor::Editor, server::ClientId};

#[action("Reload the current window")]
fn reload(editor: &mut Editor, id: ClientId) {
    editor.reload(id);
}

#[action("Clear messages")]
fn clear_messages(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_msg();
}

#[action("Sync other windows if a buffer is changed")]
fn sync_windows(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf(id);
    let clients = editor.windows().find_clients_with_buf(buf.id);

    for client in clients {
        if client == id {
            continue;
        }

        let (win, buf) = editor.win_buf_mut(client);
        win.on_buffer_changed(buf);
    }
}
