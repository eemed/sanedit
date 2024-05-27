use crate::{editor::Editor, server::ClientId};

use super::{
    char::{grapheme_category, GraphemeCategory},
    text::word_at_pos,
};

pub(crate) fn word_at_cursor(editor: &Editor, id: ClientId) -> Option<String> {
    let (win, buf) = editor.win_buf(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let range = word_at_pos(&slice, cursor)?;
    let word = buf.slice(range);
    Some(String::from(&word))
}

pub(crate) fn non_whitespace_before_cursor(editor: &Editor, id: ClientId) -> Option<String> {
    let (win, buf) = editor.win_buf(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let mut start = cursor;
    let mut graphemes = slice.graphemes_at(cursor);
    while let Some(g) = graphemes.prev() {
        use GraphemeCategory::*;
        match grapheme_category(&g) {
            EOL | Whitespace => break,
            _ => start = g.start(),
        }
    }

    let word = buf.slice(start..cursor);
    Some(String::from(&word))
}
