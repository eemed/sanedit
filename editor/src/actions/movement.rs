use crate::{common, editor::Editor, server::ClientId};

pub(crate) fn next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::next_grapheme_boundary(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::prev_grapheme_boundary(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn start_of_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::start_of_line(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn end_of_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::end_of_line(&buf.slice(..), cursor.pos());
    cursor.goto(pos);
}

pub(crate) fn start_of_buffer(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    cursor.goto(0);
}

pub(crate) fn end_of_buffer(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    cursor.goto(buf.len());
}

pub(crate) fn prev_visual_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
}

pub(crate) fn next_visual_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.view().primary_cursor();
    let last_line = win.view().height().saturating_sub(1);
    let on_last_line = cursor.y == last_line;
    if on_last_line {
        win.scroll_down(buf);
    }

    // TODO 
    // make point below cursor => y - 1
    // Make sure it its x is within the line => cmp::min(col, width)
    //
    //
    // let line = cmp::min(y + 1, last_line);
    // let pos = common::pos_at_line(&view, line, v_col)?;
    // Some((pos, v_col))
}
