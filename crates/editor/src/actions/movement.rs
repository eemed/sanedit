use std::{
    cmp::{self, min},
    sync::Arc,
};

use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{
    find_range,
    movement::{
        self, find_next_char, find_prev_char, next_grapheme_boundary, prev_grapheme_boundary,
    },
    Cursor, DisplayOptions,
};
use sanedit_messages::redraw::Point;

use crate::editor::{
    hooks::Hook,
    windows::{Jump, JumpGroup, NextKeyFunction, View},
    Editor,
};

use sanedit_server::ClientId;

use super::{hooks, ActionResult};

#[inline]
fn do_move_line<F: Fn(&PieceTreeSlice, &Cursor, &DisplayOptions) -> (u64, usize)>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
    save_jump: bool,
) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let opts = win.display_options().clone();
    let primary = win.cursors.primary_index();
    let mut jump = None;
    let mut changed = false;

    for (i, cursor) in win.cursors.cursors_mut().iter_mut().enumerate() {
        let opos = cursor.pos();
        let (pos, col) = f(&buf.slice(..), cursor, &opts);
        cursor.goto_with_col(pos, col);

        changed |= cursor.pos() != opos;

        if save_jump && i == primary && cursor.pos() != opos {
            let mark = buf.mark(opos);
            jump = Some(Jump::new(mark, None));
        }
    }

    if let Some(jump) = jump {
        win.cursor_jumps.push(JumpGroup::new(buf.id, vec![jump]));
        win.cursor_jumps.goto_start();
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
        ActionResult::Ok
    } else {
        ActionResult::Skipped
    }
}

#[inline]
fn do_move<F: Fn(&PieceTreeSlice, u64) -> u64>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
    col: Option<usize>,
    save_jump: bool,
) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let mut changed = false;
    let primary = win.cursors.primary_index();
    let mut jump = None;

    for (i, cursor) in win.cursors.cursors_mut().iter_mut().enumerate() {
        let opos = cursor.pos();
        let pos = f(&buf.slice(..), cursor.pos());
        if let Some(col) = col {
            cursor.goto_with_col(pos, col);
        } else {
            cursor.goto(pos);
        }
        changed |= cursor.pos() != opos;

        if save_jump && i == primary && cursor.pos() != opos {
            let mark = buf.mark(opos);
            jump = Some(Jump::new(mark, None));
        }
    }

    if let Some(jump) = jump {
        win.cursor_jumps.push(JumpGroup::new(buf.id, vec![jump]));
        win.cursor_jumps.goto_start();
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
        ActionResult::Ok
    } else {
        ActionResult::Skipped
    }
}

#[inline]
fn do_move_static(
    editor: &mut Editor,
    id: ClientId,
    pos: u64,
    col: Option<usize>,
    save_jump: bool,
) -> ActionResult {
    do_move(editor, id, |_, _| pos, col, save_jump)
}

#[action("Cursors: Goto to next character")]
fn next_grapheme(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, next_grapheme_boundary, None, false)
}

#[action("Cursors: Goto to previous character")]
fn prev_grapheme(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, prev_grapheme_boundary, None, false)
}

#[action("Cursors: Goto to first character on line")]
fn first_char_of_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::first_char_of_line, None, false)
}

#[action("Cursors: Goto to line start")]
fn start_of_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::start_of_line, Some(0), false)
}

#[action("Cursors: Goto to line end")]
fn end_of_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::end_of_line, Some(usize::MAX), false)
}

#[action("Cursors: Goto to buffer start")]
fn start_of_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move_static(editor, id, 0, None, true)
}

#[action("Cursors: Goto to buffer end")]
fn end_of_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    let blen = {
        let (_win, buf) = editor.win_buf(id);
        buf.len()
    };
    do_move_static(editor, id, blen, None, true)
}

#[action("Cursors: Goto to next word start")]
fn next_word_start(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::next_word_start, None, false)
}

#[action("Cursors: Goto to previous word start")]
fn prev_word_start(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::prev_word_start, None, false)
}

#[action("Cursors: Goto to next word end")]
fn next_word_end(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::next_word_end, None, false)
}

#[action("Cursors: Goto to previous word end")]
fn prev_word_end(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::prev_word_end, None, false)
}

#[action("Cursors: Goto to next paragraph")]
fn next_paragraph(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::next_paragraph, None, true)
}

#[action("Cursors: Goto to previous paragraph")]
fn prev_paragraph(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::prev_paragraph, None, true)
}

#[action("Cursors: Goto to next line")]
fn next_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move_line(editor, id, movement::next_line, false)
}

#[action("Cursors: Goto to previous line")]
fn prev_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move_line(editor, id, movement::prev_line, false)
}

const PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

pub fn pair_for(ch: char) -> Option<(char, char)> {
    for (a, b) in PAIRS.iter() {
        if a == &ch || b == &ch {
            return Some((*a, *b));
        }
    }

    None
}

#[action("Cursors: Goto to matching pair")]
fn goto_matching_pair(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let mut chars = slice.chars_at(pos);
    let (_, _, ch) = getf!(chars.next());
    let (start, end) = pair_for(ch).unwrap_or((ch, ch));
    let mut buf1 = [0u8; 4];
    let mut buf2 = [0u8; 4];
    let start = start.encode_utf8(&mut buf1);
    let end = end.encode_utf8(&mut buf2);

    let range = getf!(find_range(&slice, pos, start, end, true));
    if range.start == pos {
        do_move_static(editor, id, range.end - end.len() as u64, None, true)
    } else {
        do_move_static(editor, id, range.start, None, true)
    }
}

#[action("Cursors: Find next char on line")]
fn find_next_char_on_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.next_key_handler = Some(NextKeyFunction(Arc::new(|editor, id, event| {
        let ch = match event.key() {
            sanedit_messages::key::Key::Char(ch) => *ch,
            _ => return ActionResult::Failed,
        };
        do_move(
            editor,
            id,
            |slice, pos| {
                let npos = min(pos + 1, slice.len());
                let next = find_next_char(slice, npos, ch, true);
                next.unwrap_or(pos)
            },
            None,
            false,
        );
        let (win, _buf) = editor.win_buf_mut(id);
        win.search.on_line_char_search = Some(ch);
        ActionResult::Ok
    })));
    ActionResult::Ok
}

#[action("Cursors: Find previous char on line")]
fn find_prev_char_on_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.next_key_handler = Some(NextKeyFunction(Arc::new(|editor, id, event| {
        let ch = match event.key() {
            sanedit_messages::key::Key::Char(ch) => *ch,
            _ => return ActionResult::Failed,
        };
        do_move(
            editor,
            id,
            |slice, pos| find_prev_char(slice, pos, ch, true).unwrap_or(pos),
            None,
            false,
        );
        let (win, _buf) = editor.win_buf_mut(id);
        win.search.on_line_char_search = Some(ch);
        ActionResult::Ok
    })));
    ActionResult::Ok
}

#[action("Cursors: Next searched char on line")]
fn next_searched_char(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(ch) = win.search.on_line_char_search {
        do_move(
            editor,
            id,
            |slice, pos| {
                let npos = min(pos + 1, slice.len());
                let next = find_next_char(slice, npos, ch, true);
                next.unwrap_or(pos)
            },
            None,
            false,
        )
    } else {
        ActionResult::Skipped
    }
}

#[action("Cursors: Previous searched char on line")]
fn prev_searched_char(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(ch) = win.search.on_line_char_search {
        do_move(
            editor,
            id,
            |slice, pos| find_prev_char(slice, pos, ch, true).unwrap_or(pos),
            None,
            false,
        )
    } else {
        ActionResult::Skipped
    }
}

#[action("Cursors: Goto to previous character on the same line")]
fn prev_grapheme_on_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::prev_grapheme_on_line, None, false)
}

#[action("Cursors: Goto to next character on the same line")]
fn next_grapheme_on_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    do_move(editor, id, movement::next_grapheme_on_line, None, false)
}

#[action("Cursors: Goto to previous visual line")]
pub(crate) fn prev_visual_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);

    // If multicursor use lines
    let multi_cursor = win.cursors.len() > 1;
    if multi_cursor {
        prev_line.execute(editor, id);
        return ActionResult::Ok;
    }

    win.view_to_cursor(buf);
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = getf!(win.view().point_at_pos(cursor_pos));
    let cursor_at_start = cursor_point.y == 0;
    let view_at_start = win.view().at_start();

    if cursor_at_start && view_at_start {
        return ActionResult::Ok;
    }

    if cursor_at_start && !view_at_start {
        // We are at the top line already, but view can be scrolled up
        win.scroll_up_n(buf, 1);
    }

    let cursor = win.cursors.primary();
    let view = win.view();
    if let Some((pos, col)) = prev_visual_line_impl(view, cursor) {
        win.cursors.cursors_mut().primary().goto_with_col(pos, col);
        hooks::run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}

// Moves cursor one visual line up, but will not change the view.
// Before using this you should check if the view can be scrolled
// up and do so. returns wether cursor was moved.
fn prev_visual_line_impl(view: &View, cursor: &Cursor) -> Option<(u64, usize)> {
    let cursor_pos = cursor.pos();
    let cursor_point = view.point_at_pos(cursor_pos)?;
    if cursor_point.y == 0 {
        return None;
    }

    // Targets where we want to end up
    let target_line = cursor_point.y.saturating_sub(1);
    let target_col = cursor.column().unwrap_or(cursor_point.x);

    // Last character on the target line
    let max_col = view
        .last_non_empty_cell(target_line)
        .map(|point| point.x)
        .unwrap_or(0);
    // Column where there exists a character
    let col = target_col.min(max_col);

    let pos = view
        .pos_at_point(Point {
            x: col,
            y: target_line,
        })
        .unwrap_or(0);
    Some((pos, target_col))
}

#[action("Cursors: Goto to next visual line")]
fn next_visual_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);

    // If multicursor use lines
    let multi_cursor = win.cursors.len() > 1;
    if multi_cursor {
        next_line.execute(editor, id);
        return ActionResult::Ok;
    }

    win.view_to_cursor(buf);
    let cursor_pos = win.cursors().primary().pos();
    let cursor_point = getf!(win.view().point_at_pos(cursor_pos));
    let last_line = win.view().height().saturating_sub(1);
    let cursor_at_end = cursor_point.y == last_line;
    let view_at_end = win.view().at_end();

    if cursor_at_end && view_at_end {
        return ActionResult::Ok;
    }

    // Make sure we have atleast one extra line to down to
    if cursor_at_end && !view_at_end {
        win.scroll_down_n(buf, 1);
    }

    let view = win.view();
    let cursor = win.cursors.primary();
    if let Some((pos, col)) = next_visual_line_impl(view, cursor, buf.len()) {
        win.cursors.cursors_mut().primary().goto_with_col(pos, col);
        hooks::run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}

// Moves cursor one visual line down, but will not change the view.
//  Before using this you should check if the view can be
// scrolled down and do so. returns wether cursor was moved.
fn next_visual_line_impl(view: &View, cursor: &Cursor, buf_len: u64) -> Option<(u64, usize)> {
    let cursor_pos = cursor.pos();
    let cursor_point = view.point_at_pos(cursor_pos)?;
    let last_line = view.height().saturating_sub(1);
    let target_line = cmp::min(cursor_point.y + 1, last_line);

    let max_col = view.last_non_empty_cell(target_line).map(|point| point.x)?;
    let cursor_col = cursor.column().unwrap_or(cursor_point.x);
    let col = cursor_col.min(max_col);

    let pos = view
        .pos_at_point(Point {
            x: col,
            y: target_line,
        })
        .unwrap_or(buf_len);

    Some((pos, cursor_col))
}
