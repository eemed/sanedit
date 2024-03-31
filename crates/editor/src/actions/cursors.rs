use sanedit_buffer::Searcher;
use sanedit_messages::redraw::Point;

use crate::{
    common::{
        movement::{next_line, next_line_start, prev_line, start_of_line},
        window::pos_at_point,
    },
    editor::{windows::Cursor, Editor},
    server::ClientId,
};

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
    let last_search = win.search.prompt.input();
    let ppos = win.cursors.primary().pos();

    let searcher = Searcher::new(last_search.as_bytes());
    let slice = buf.slice(ppos..);
    let mut iter = searcher.find_iter(&slice);
    if let Some(mut mat) = iter.next() {
        mat.start += ppos;
        mat.end += ppos;

        let selecting = win.primary_cursor().selection().is_some();
        if selecting {
            win.cursors.push_primary(Cursor::new_select(&mat));
        } else {
            let cursor = win.cursors.primary_mut();
            *cursor = Cursor::new_select(&mat);
        }
    }
}

#[action("Create a new cursor on each search match")]
fn new_to_all_search_matches(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    // win.cursors.remove_secondary_cursors();

    let last_search = win.search.prompt.input();
    let searcher = Searcher::new(last_search.as_bytes());
    let slice = buf.slice(..);
    let mut iter = searcher.find_iter(&slice);

    while let Some(mat) = iter.next() {
        let selecting = win.primary_cursor().selection().is_some();
        if selecting {
            win.cursors.push_primary(Cursor::new_select(&mat));
        } else {
            let cursor = win.cursors.primary_mut();
            *cursor = Cursor::new_select(&mat);
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
    // TODO hooks?
    for cursor in win.cursors.cursors_mut() {
        let slice = buf.slice(..);
        let pos = cursor.pos();
        let start = start_of_line(&slice, pos);
        let end = next_line_start(&slice, pos);
        if start == end {
            continue;
        }

        cursor.goto(start);
        cursor.anchor();
        cursor.goto(end);
    }
}

#[action("Merge overlapping cursors")]
fn merge_overlapping_cursors(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.merge_overlapping();
}
