use std::cmp;

use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{
    movement::{self, next_grapheme_boundary, prev_grapheme_boundary},
    pairs, Cursor, DisplayOptions,
};
use sanedit_messages::redraw::Point;

use crate::{
    editor::{hooks::Hook, Editor},
    server::ClientId,
};

use super::hooks;

#[inline]
fn do_move_line<F: Fn(&PieceTreeSlice, &Cursor, &DisplayOptions) -> (u64, usize)>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
) {
    let mut changed = false;

    let (win, buf) = editor.win_buf_mut(id);
    let opts = win.display_options().clone();
    for cursor in win.cursors.cursors_mut() {
        let opos = cursor.pos();
        let (pos, col) = f(&buf.slice(..), cursor, &opts);
        cursor.goto_with_col(pos, col);

        changed |= cursor.pos() != opos;
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
    }
}

#[inline]
fn do_move<F: Fn(&PieceTreeSlice, u64) -> u64>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
    col: Option<usize>,
) {
    let mut changed = false;

    let (win, buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        let opos = cursor.pos();
        let pos = f(&buf.slice(..), cursor.pos());
        if let Some(col) = col {
            cursor.goto_with_col(pos, col);
        } else {
            cursor.goto(pos);
        }
        changed |= cursor.pos() != opos;
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
    }
}

#[inline]
fn do_move_static(editor: &mut Editor, id: ClientId, pos: u64, col: Option<usize>) {
    do_move(editor, id, |_, _| pos, col);
}

#[action("Move cursor(s) to the next character")]
fn next_grapheme(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, next_grapheme_boundary, None);
}

#[action("Move cursor(s) to the previous character")]
fn prev_grapheme(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, prev_grapheme_boundary, None);
}

#[action("Move cursor(s) to the first character of the line")]
fn first_char_of_line(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::first_char_of_line, None);
}

#[action("Move cursor(s) to the start of the line")]
fn start_of_line(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::start_of_line, Some(0));
}

#[action("Move cursor(s) to the end of the line")]
fn end_of_line(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::end_of_line, Some(usize::MAX));
}

#[action("Move cursor(s) to the beginning of the buffer")]
fn start_of_buffer(editor: &mut Editor, id: ClientId) {
    do_move_static(editor, id, 0, None);
}

#[action("Move cursor(s) to the end of the buffer")]
fn end_of_buffer(editor: &mut Editor, id: ClientId) {
    let blen = {
        let (_win, buf) = editor.win_buf(id);
        buf.len()
    };
    do_move_static(editor, id, blen, None);
}

#[action("Move cursor(s) to the start of the next word")]
fn next_word_start(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::next_word_start, None);
}

#[action("Move cursor(s) to the start of the previous word")]
fn prev_word_start(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::prev_word_start, None);
}

#[action("Move cursor(s) to the end of the next word")]
fn next_word_end(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::next_word_end, None);
}

#[action("Move cursor(s) to the end of the previous word")]
fn prev_word_end(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::prev_word_end, None);
}

#[action("Move cursor(s) to the next paragraph")]
fn next_paragraph(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::next_paragraph, None);
}

#[action("Move cursor(s) to the previous paragraph")]
fn prev_paragraph(editor: &mut Editor, id: ClientId) {
    do_move(editor, id, movement::prev_paragraph, None);
}

#[action("Move cursor(s) to the previous visual line")]
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

#[action("Move cursor(s) to the next visual line")]
fn next_visual_line(editor: &mut Editor, id: ClientId) {
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

#[action("Move cursor(s) to the next line")]
fn next_line(editor: &mut Editor, id: ClientId) {
    do_move_line(editor, id, movement::next_line);
}

#[action("Move cursor(s) to the previous line")]
fn prev_line(editor: &mut Editor, id: ClientId) {
    do_move_line(editor, id, movement::prev_line);
}

#[action("Move cursor(s) to matching bracket pair")]
fn goto_matching_pair(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
    let slice = buf.slice(..);
    if let Some(pos) = pairs::matching_pair(&slice, pos) {
        do_move_static(editor, id, pos, None);
    }
}
