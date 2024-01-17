use crate::{common::text_objects::find_range, editor::Editor, server::ClientId};

fn select_impl(editor: &mut Editor, id: ClientId, start: &str, end: &str, include: bool) {
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);

    for cursor in win.cursors.cursors_mut() {
        let pos = cursor.pos();
        let range = find_range(&slice, pos, start, end, include);

        if let Some(range) = range {
            if !range.is_empty() {
                cursor.select(range);
            }
        }
    }
    win.view_to_cursor(buf);
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
