use std::{fs::File, path::Path};

use crate::{
    common::file::FileMetadata,
    editor::{
        buffers::Buffer,
        options::{Convert, EditorOptions},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn scroll_down(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.scroll_down_n(buf, 1);
}

pub(crate) fn scroll_up(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.scroll_up_n(buf, 1);
}

pub(crate) fn open_file(editor: &mut Editor, id: ClientId, path: impl AsRef<Path>) {
    let opts = &editor.options;
    let (win, buf) = editor.get_win_buf_mut(id);
    win.open_file(path, opts);
}
