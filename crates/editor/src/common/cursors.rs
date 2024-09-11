use crate::{editor::Editor, server::ClientId};

use sanedit_core::{grapheme_category, word_at_pos, GraphemeCategory};

pub(crate) fn word_at_cursor(editor: &Editor, id: ClientId) -> Option<String> {
    let (win, buf) = editor.win_buf(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = word_at_pos(&slice, cursor)?;
    let word = buf.slice(range);
    Some(String::from(&word))
}

pub(crate) fn word_before_cursor(editor: &Editor, id: ClientId) -> Option<(u64, u64, String)> {
    let (win, buf) = editor.win_buf(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let mut start = cursor;
    let mut graphemes = slice.graphemes_at(cursor);
    while let Some(g) = graphemes.prev() {
        use GraphemeCategory::*;
        match grapheme_category(&g) {
            Word => start = g.start(),
            _ => break,
        }
    }

    let word = buf.slice(start..cursor);
    let word = String::from(&word);
    Some((start, cursor, word))
}
