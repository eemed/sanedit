use sanedit_messages::redraw::Point;

use crate::{
    common::window::pos_at_point,
    editor::{hooks::Hook, Editor},
};

use sanedit_server::ClientId;

use sanedit_core::{
    movement::{next_line, prev_line},
    Cursor, PTSearcher,
};

use super::{
    hooks::{self, run},
    movement,
};

#[action("Start selection and jump to next word")]
fn select_to_next_word(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        if !cursor.is_selecting() {
            cursor.start_selection();
        }
    }
    movement::next_word_end.execute(editor, id);
}

#[action("Start selection and jump to prev word")]
fn select_to_prev_word(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        if !cursor.is_selecting() {
            cursor.start_selection();
        }
    }
    movement::prev_word_start.execute(editor, id);
}

#[action("New cursor on the next line")]
fn new_cursor_to_next_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = next_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

#[action("New cursor on the previous line")]
fn new_cursor_to_prev_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = prev_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

#[action("New cursor on the next search match")]
fn new_cursor_to_next_search_match(editor: &mut Editor, id: ClientId) {
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

#[action("New cursor on each search match")]
fn new_cursor_to_all_search_matches(editor: &mut Editor, id: ClientId) {
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
    let iter = searcher.find_iter(&slice);

    for mat in iter {
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
    win.cursors.remove_except_primary();
    if let Some(pos) = pos_at_point(win, point) {
        let primary = win.cursors.primary_mut();
        primary.stop_selection();
        primary.goto(pos);
        hooks::run(editor, id, Hook::CursorMoved);
    }
}

#[action("Select")]
fn start_selection(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.start_selection();
}

#[action("Unselect")]
fn stop_selection(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.stop_selection();
}

#[action("Keep only primary cursor")]
fn keep_only_primary(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    if win.cursors.cursors().iter().any(|c| c.is_selecting()) {
        win.cursors.stop_selection();
    } else {
        win.cursors.remove_except_primary();

        let (win, _buf) = editor.win_buf_mut(id);
        win.cursors.primary_mut().stop_selection();
    }
}

#[action("Swap cursor in selection")]
fn swap_selection_dir(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.swap_selection_dir();
}

#[action("Remove primary cursor")]
fn remove_primary_cursor(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.remove_primary();
}

#[action("Make next cursor primary")]
fn make_next_cursor_primary(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_next();
}

#[action("Make previous cursor primary")]
fn make_prev_cursor_primary(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_prev();
}

#[action("Merge overlapping cursors")]
fn merge_overlapping_cursors(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.merge_overlapping();
}

#[action("Remove cursor selections")]
fn remove_cursor_selections(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    match win.remove_cursor_selections(buf) {
        Ok(true) => {
            let hook = Hook::BufChanged(buf.id);
            run(editor, id, hook);
        }
        _ => {}
    }
}
