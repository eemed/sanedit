use std::path::Path;

use crate::{
    common::file::FileMetadata,
    editor::{options::EditorOptions, Editor},
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
    let (win, buf) = editor.get_win_buf_mut(id);
    let path = path.as_ref();
    match FileMetadata::try_from(path) {
        Ok(m) => {
            let EditorOptions {
                big_file_threshold_bytes,
                convert_small,
                convert_big,
                ..
            } = editor.options;
            let is_utf8 = m.encoding == encoding_rs::UTF_8;
            let is_big = m.size >= editor.options.big_file_threshold_bytes;

            match (is_utf8, is_big) {
                (false, true) => {
                    // according to convert_big, to temp file
                }
                (false, false) => {
                    // according to convert_small, in memory
                }
                (true, true) => {
                    // Open file backed buffer
                }
                (true, false) => {
                    // Open in memory buffer
                }
                _ => {
                    // open in memory
                }
            }
        }
        Err(e) => {
            log::error!(
                "Failed to read file {} metadata {}",
                path.to_string_lossy(),
                e
            );
        }
    }
}
