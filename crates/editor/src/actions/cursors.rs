use sanedit_messages::redraw::Point;

use crate::{
    common::{
        cursors::word_at_cursor,
        movement::{end_of_line, next_line, prev_line, start_of_line},
        search::PTSearcher,
        window::pos_at_point,
    },
    editor::{hooks::Hook, windows::Cursor, Editor},
    server::ClientId,
};

use super::{hooks, search};

#[action("Create a new cursor on the next line")]
fn new_next_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = next_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

#[action("Create a new cursor on the previous line")]
fn new_prev_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = prev_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

#[action("Create a new cursor on the next search match")]
fn new_to_next_search_match(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let Some(last_search) = win.search.last_search() else {
        return;
    };
    let ppos = win.cursors.primary().pos();

    let Ok(searcher) = PTSearcher::new(&last_search.pattern, last_search.dir, last_search.kind)
    else {
        return;
    };
    let slice = buf.slice(ppos..);
    let mut iter = searcher.find_iter(&slice);
    if let Some(mat) = iter.next() {
        let mut range = mat.range();
        range.start += ppos;
        range.end += ppos;

        let selecting = win.primary_cursor().selection().is_some();
        if selecting {
            win.cursors.push_primary(Cursor::new_select(&range));
        } else {
            let cursor = win.cursors.primary_mut();
            *cursor = Cursor::new_select(&range);
        }
    }
}

#[action("Create a new cursor on each search match")]
fn new_to_all_search_matches(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    // win.cursors.remove_secondary_cursors();

    let Some(last_search) = win.search.last_search() else {
        win.warn_msg("No last search pattern");
        return;
    };

    let Ok(searcher) = PTSearcher::new(&last_search.pattern, last_search.dir, last_search.kind)
    else {
        return;
    };
    let slice = buf.slice(..);
    let mut iter = searcher.find_iter(&slice);

    while let Some(mat) = iter.next() {
        let range = mat.range();
        let selecting = win.primary_cursor().selection().is_some();
        if selecting {
            win.cursors.push_primary(Cursor::new_select(&range));
        } else {
            let cursor = win.cursors.primary_mut();
            *cursor = Cursor::new_select(&range);
        }
    }
}

pub(crate) fn new_to_point(editor: &mut Editor, id: ClientId, point: Point) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(pos) = pos_at_point(win, point) {
        win.cursors.push(Cursor::new(pos));
    }
}

pub(crate) fn goto_position(editor: &mut Editor, id: ClientId, point: Point) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.remove_secondary_cursors();
    if let Some(pos) = pos_at_point(win, point) {
        let primary = win.cursors.primary_mut();
        primary.unanchor();
        primary.goto(pos);
        hooks::run(editor, id, Hook::CursorMoved);
    }
}

#[action("Start selecting")]
fn start_selection(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.start_selection();
}

#[action("Keep only the primary cursor")]
fn remove_secondary(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.remove_secondary_cursors();
}

#[action("Swap cursor position inside the selection")]
fn swap_selection_dir(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.swap_selection_dir();
}

#[action("Remove primary cursor")]
fn remove(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.remove_primary();
}

#[action("Make next cursor primary")]
fn next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_next();
}

#[action("Make previous cursor primary")]
fn prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_prev();
}

#[action("Select line")]
fn select_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        let slice = buf.slice(..);
        let pos = cursor.pos();
        let start = start_of_line(&slice, pos);
        let end = end_of_line(&slice, pos);
        // let end = next_line_start(&slice, pos);
        if start == end {
            continue;
        }

        cursor.goto(start);
        cursor.anchor();
        cursor.goto(end);
        cursor.set_column(usize::MAX);
    }
}

#[action("Merge overlapping cursors")]
fn merge_overlapping_cursors(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.merge_overlapping();
}

#[action("Restore single non selection cursor")]
fn single_non_selecting(editor: &mut Editor, id: ClientId) {
    remove_secondary.execute(editor, id);

    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_mut().unanchor();
}
