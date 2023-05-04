use std::cmp;

use sanedit_buffer::piece_tree::PieceTreeSlice;
use sanedit_messages::redraw::Point;

use crate::{common, editor::Editor, server::ClientId};

fn do_move<F: Fn(&PieceTreeSlice, usize) -> usize>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
    col: Option<usize>,
) {
    let (win, buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        let pos = f(&buf.slice(..), cursor.pos());
        if let Some(col) = col {
            cursor.goto_with_col(pos, col);
        } else {
            cursor.goto(pos);
        }
    }
    win.view_to_cursor(buf);
}

fn do_move_static(editor: &mut Editor, id: ClientId, pos: usize, col: Option<usize>) {
    do_move(editor, id, |_, _| pos, col);
}

pub(crate) fn next_grapheme(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::next_grapheme_boundary, None);
}

pub(crate) fn prev_grapheme(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::prev_grapheme_boundary, None);
}

pub(crate) fn start_of_line(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::start_of_line, Some(0));
}

pub(crate) fn end_of_line(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::end_of_line, Some(usize::MAX));
}

pub(crate) fn start_of_buffer(editor: &mut Editor, id: ClientId) {
    do_move_static(editor, id, 0, None);
}

pub(crate) fn end_of_buffer(editor: &mut Editor, id: ClientId) {
    let blen = {
        let (win, buf) = editor.win_buf(id);
        buf.len()
    };
    do_move_static(editor, id, blen, None);
}

pub(crate) fn next_word_start(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::next_word_start, None);
}

pub(crate) fn prev_word_start(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::prev_word_start, None);
}

pub(crate) fn next_paragraph(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::next_paragraph, None);
}

pub(crate) fn prev_paragraph(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, common::movement::prev_paragraph, None);
}

pub(crate) fn prev_visual_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
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
        win.scroll_up_n(buf, 1);
    }

    prev_visual_line_impl(editor, id);
}

// Moves cursor one visual line up, but will not change the view.
// Before using this you should check if the view can be scrolled
// up and do so. returns wether cursor was moved.
fn prev_visual_line_impl(editor: &mut Editor, id: ClientId) -> bool {
    let (win, _buf) = editor.win_buf_mut(id);
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
    let col = target_col.min(max_col);

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
    let (win, buf) = editor.win_buf_mut(id);
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

    // Make sure we have atleast one extra line to down to
    if cursor_at_end && !view_at_end {
        win.scroll_down_n(buf, 1);
    }

    next_visual_line_impl(editor, id);
}

// Moves cursor one visual line down, but will not change the view.
//  Before using this you should check if the view can be
// scrolled down and do so. returns wether cursor was moved.
fn next_visual_line_impl(editor: &mut Editor, id: ClientId) -> bool {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = win
        .view()
        .point_at_pos(cursor_pos)
        .expect("cursor not in view");
    let last_line = win.view().height().saturating_sub(1);
    let target_line = cmp::min(cursor_point.y + 1, last_line);

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
    let col = cursor_col.min(max_col);

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
