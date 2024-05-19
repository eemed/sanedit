use crate::{editor::Editor, server::ClientId};

use super::text::word_at_pos;

pub(crate) fn word_at_cursor(editor: &Editor, id: ClientId) -> Option<String> {
    let (win, buf) = editor.win_buf(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = word_at_pos(&slice, cursor)?;
    let word = buf.slice(range);
    Some(String::from(&word))
}

pub(crate) fn word_before_cursor(editor: &Editor, id: ClientId) -> Option<String> {
    let (win, buf) = editor.win_buf(id);
    let cursor = win.cursors.primary().pos().saturating_sub(1);
    let slice = buf.slice(..);
    log::info!("Word at: {cursor}");
    let range = word_at_pos(&slice, cursor)?;
    log::info!("range: {range:?}");
    let word = buf.slice(range);
    Some(String::from(&word))
}
