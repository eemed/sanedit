use crate::{
    common::{text::word_at_pos, text_objects::find_range},
    editor::{hooks::Hook, Editor},
    server::ClientId,
};

use super::hooks;

fn select_impl(editor: &mut Editor, id: ClientId, start: &str, end: &str, include: bool) {
    let mut changed = false;
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);

    for cursor in win.cursors.cursors_mut() {
        let pos = cursor.pos();
        let range = find_range(&slice, pos, start, end, include);

        if let Some(range) = range {
            if !range.is_empty() {
                cursor.select(range);
                changed = true;
            }
        }
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
    }
}

#[action("Select in curly brackets")]
fn select_in_curly(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "{", "}", false);
}

#[action("Select including curly brackets")]
fn select_curly(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "{", "}", true);
}

#[action("Select in parentheses")]
fn select_in_parens(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "(", ")", false);
}

#[action("Select including parentheses")]
fn select_parens(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "(", ")", true);
}

#[action("Select in square brackets")]
fn select_in_square(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "[", "]", false);
}

#[action("Select including square brackets")]
fn select_square(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "[", "]", true);
}

#[action("Select in angle brackets")]
fn select_in_angle(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "<", ">", false);
}

#[action("Select including angle brackets")]
fn select_angle(editor: &mut Editor, id: ClientId) {
    select_impl(editor, id, "<", ">", true);
}

#[action("Select word under cursor")]
fn select_word(editor: &mut Editor, id: ClientId) {
    let mut changed = false;
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);

    for cursor in win.cursors.cursors_mut() {
        let pos = cursor.pos();
        let range = word_at_pos(&slice, pos);

        if let Some(range) = range {
            if !range.is_empty() {
                cursor.select(range);
                changed = true;
            }
        }
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
    }
}
