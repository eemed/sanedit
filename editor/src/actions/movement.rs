use std::cmp;

use sanedit_messages::redraw::Point;

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
    cursor.goto_with_col(pos, 0);
}

pub(crate) fn end_of_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor = win.primary_cursor_mut();
    let pos = common::movement::end_of_line(&buf.slice(..), cursor.pos());
    cursor.goto_with_col(pos, usize::MAX);
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
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = win
        .view()
        .point_at_pos(cursor_pos)
        .expect("cursor not in view");
    let cursor_at_start = cursor_point.y == 0;
    let view_at_start = win.view().at_start();

    if cursor_at_start && view_at_start {
        return;
    }

    if cursor_at_start && !view_at_start {
        // We are at the top line already, but view can be scrolled up
        win.scroll_up(buf);
    }

    prev_visual_line_impl(editor, id);
}

// Moves cursor one visual line up, but will not change the view.
// Before using this you should check if the view can be scrolled
// up and do so. returns wether cursor was moved.
fn prev_visual_line_impl(editor: &mut Editor, id: ClientId) -> bool {
    let (win, _buf) = editor.get_win_buf_mut(id);
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = win
        .view()
        .point_at_pos(cursor_pos)
        .expect("cursor not in view");
    if cursor_point.y == 0 {
        return false;
    }

    // Targets where we want to end up
    let target_line = cursor_point.y.saturating_sub(1);
    let target_col = win.primary_cursor().column().unwrap_or(cursor_point.x);

    // Last character on the target line
    let max_col = win
        .view()
        .last_non_empty_cell(target_line)
        .map(|point| point.x)
        .unwrap_or(0);
    // Column where there exists a character
    let col = cmp::min(max_col, target_col);

    let pos = win
        .view()
        .pos_at_point(Point {
            x: col,
            y: target_line,
        })
        .unwrap_or(0);
    win.primary_cursor_mut().goto_with_col(pos, target_col);
    true
}

pub(crate) fn next_visual_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = win
        .view()
        .point_at_pos(cursor_pos)
        .expect("cursor not in view");
    let last_line = win.view().height().saturating_sub(1);
    let cursor_at_end = cursor_point.y == last_line;
    let view_at_end = win.view().at_end();

    if cursor_at_end && view_at_end {
        return;
    }

    if cursor_at_end && !view_at_end {
        win.scroll_down(buf);
    }

    next_visual_line_impl(editor, id);
}

// Moves cursor one visual line down, but will not change the view.
//  Before using this you should check if the view can be
// scrolled down and do so. returns wether cursor was moved.
fn next_visual_line_impl(editor: &mut Editor, id: ClientId) -> bool {
    let (win, buf) = editor.get_win_buf_mut(id);
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = win
        .view()
        .point_at_pos(cursor_pos)
        .expect("cursor not in view");
    let last_line = win.view().height().saturating_sub(1);
    let target_line = cmp::min(last_line, cursor_point.y + 1);

    let max_col = match win
        .view()
        .last_non_empty_cell(target_line)
        .map(|point| point.x)
    {
        Some(n) => n,
        // No cursor placeable cell on target line
        None => return false,
    };

    let cursor_col = win.primary_cursor().column().unwrap_or(cursor_point.x);
    let col = cmp::min(max_col, cursor_col);

    let pos = win
        .view()
        .pos_at_point(Point {
            x: col,
            y: target_line,
        })
        .unwrap_or(buf.len());

    win.primary_cursor_mut().goto_with_col(pos, cursor_col);
    true
}
