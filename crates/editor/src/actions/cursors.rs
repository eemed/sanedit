use sanedit_buffer::MarkResult;
use sanedit_messages::redraw::Point;
use sanedit_utils::ring::Ref;

use crate::{
    common::window::pos_at_point,
    editor::{
        buffers::{BufferId, Buffers},
        hooks::Hook,
        windows::{Jumps, Zone},
        Editor,
    },
};

use sanedit_server::ClientId;

use sanedit_core::{
    movement::{next_line, prev_line},
    Cursor, Searcher,
};

use super::{
    hooks::{self, run},
    movement,
};

#[action("Cursors: Select next word")]
fn select_to_next_word(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        if !cursor.is_selecting() {
            cursor.start_selection();
        }
    }
    movement::next_word_end.execute(editor, id);
}

#[action("Cursors: Select previous word")]
fn select_to_prev_word(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        if !cursor.is_selecting() {
            cursor.start_selection();
        }
    }
    movement::prev_word_start.execute(editor, id);
}

#[action("Cursors: New on next line")]
fn new_cursor_to_next_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = next_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

#[action("Cursors: New on previous line")]
fn new_cursor_to_prev_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = prev_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
}

#[action("Cursors: New on next search match")]
fn new_cursor_to_next_search_match(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let Some(last_search) = win.search.last_search() else {
        return;
    };
    let ppos = win.cursors.primary().pos();

    let Ok(searcher) = Searcher::new(&last_search.pattern, last_search.kind) else {
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

#[action("Cursors: New on each search match")]
fn new_cursor_to_all_search_matches(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    // win.cursors.remove_secondary_cursors();

    let Some(last_search) = win.search.last_search() else {
        win.warn_msg("No last search pattern");
        return;
    };

    let Ok(searcher) = Searcher::new(&last_search.pattern, last_search.kind) else {
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

#[action("Cursors: Start selection")]
fn start_selection(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.start_selection();
}

#[action("Cursors: Cancel selection")]
fn stop_selection(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.stop_selection();
}

#[action("Cursors: Remove secondary cursors")]
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

#[action("Cursors: Swap selection direction")]
fn swap_selection_dir(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.swap_selection_dir();
}

#[action("Cursors: Remove primary cursor")]
fn remove_primary_cursor(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.remove_primary();
}

#[action("Cursors: Make next cursor primary")]
fn make_next_cursor_primary(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_next();
}

#[action("Cursors: Make previous cursor primary")]
fn make_prev_cursor_primary(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.primary_prev();
}

#[action("Cursors: Merge overlapping")]
fn merge_overlapping_cursors(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.merge_overlapping();
}

#[action("Cursors: Remove selections")]
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

#[action("Cursors: New cursors on line starts")]
fn cursors_to_lines_start(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors_to_lines_start(buf);
    hooks::run(editor, id, Hook::CursorMoved);
}

#[action("Cursors: New cursors on line ends")]
fn cursors_to_lines_end(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors_to_lines_end(buf);
    hooks::run(editor, id, Hook::CursorMoved);
}

#[action("Cursors: Goto to previous change")]
fn jump_prev_change(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if win.cursors_to_prev_change(buf) {
        run(editor, id, Hook::CursorMoved)
    }
}

#[action("Cursors: Goto to next change")]
fn jump_next_change(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if win.cursors_to_next_change(buf) {
        run(editor, id, Hook::CursorMoved)
    }
}

fn find_prev_jump(
    cursor_jumps: &Jumps,
    buffers: &Buffers,
    original_bid: BufferId,
) -> Option<(Ref, BufferId)> {
    // for example iter backwards
    // until mark is Found
    // or
    //  buf changes => iter until mark is Found
    //      or
    //      if not goto Deleted marker once
    //
    // This would go to previous buffers even if mark is not found
    // but skip all other deleted markers

    let mut iter = cursor_jumps.iter();
    let mut previous = None;

    while let Some((cursor, group)) = iter.prev() {
        let gbid = group.buffer_id();

        // Return if marks are found
        if let Some(buf) = buffers.get(gbid) {
            let found = group.jumps().iter().all(|jump| {
                let mark = buf.mark_to_pos(jump.start());
                matches!(mark, MarkResult::Found(_))
            });

            if found {
                return Some((cursor, gbid));
            }
        }

        if let Some((_, pbid)) = &previous {
            // We have went to next buffer
            // and current entry is also in another buffer
            if *pbid != original_bid && *pbid != gbid {
                return previous;
            }
        }

        previous = Some((cursor, gbid));
    }

    if let Some((_, pbid)) = &previous {
        if *pbid != original_bid {
            return previous;
        }
    }

    None
}

fn find_next_jump(
    cursor_jumps: &Jumps,
    buffers: &Buffers,
    original_bid: BufferId,
) -> Option<(Ref, BufferId)> {
    // for example iter backwards
    // until mark is Found
    // or
    //  buf changes => iter until mark is Found
    //      or
    //      if not goto Deleted marker once
    //
    // This would go to previous buffers even if mark is not found
    // but skip all other deleted markers

    let mut iter = cursor_jumps.iter();
    let mut previous = None;

    while let Some((cursor, group)) = iter.next() {
        let gbid = group.buffer_id();

        // Return if marks are found
        if let Some(buf) = buffers.get(gbid) {
            let found = group.jumps().iter().all(|jump| {
                let mark = buf.mark_to_pos(jump.start());
                matches!(mark, MarkResult::Found(_))
            });

            if found {
                return Some((cursor, gbid));
            }
        }

        if let Some((_, pbid)) = &previous {
            // We have went to next buffer
            // and current entry is also in another buffer
            if *pbid != original_bid && *pbid != gbid {
                return previous;
            }
        }

        previous = Some((cursor, gbid));
    }

    if let Some((_, pbid)) = &previous {
        if *pbid != original_bid {
            return previous;
        }
    }

    None
}

#[action("Cursors: Goto previous jump")]
fn jump_prev(editor: &mut Editor, id: ClientId) {
    let (win, buf) = win_buf!(editor, id);
    let bid = buf.id;
    let (cursor, next_bid) = get!(find_prev_jump(&win.cursor_jumps, &editor.buffers, bid));

    if next_bid != bid {
        run(editor, id, Hook::BufLeave(bid));
    }

    let (win, _buf) = win_buf!(editor, id);
    let buf = get!(editor.buffers.get(next_bid));
    win.goto_cursor_jump(&cursor, buf);
    if next_bid != bid {
        run(editor, id, Hook::BufEnter(next_bid));
    }
    run(editor, id, Hook::CursorMoved)
}

#[action("Cursors: Goto next jump")]
fn jump_next(editor: &mut Editor, id: ClientId) {
    let (win, buf) = win_buf!(editor, id);
    let bid = buf.id;
    let (cursor, next_bid) = get!(find_next_jump(&win.cursor_jumps, &editor.buffers, bid));

    if next_bid != bid {
        run(editor, id, Hook::BufLeave(bid));
    }

    let (win, _buf) = win_buf!(editor, id);
    let buf = get!(editor.buffers.get(next_bid));
    win.goto_cursor_jump(&cursor, buf);
    if next_bid != bid {
        run(editor, id, Hook::BufEnter(next_bid));
    }
    run(editor, id, Hook::CursorMoved)
}

#[action("Cursors: Goto to top of view")]
fn cursor_to_view_top(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursor_to_view_zone(Zone::Top) {
        run(editor, id, Hook::CursorMoved)
    }
}

#[action("Cursors: Goto to middle of view")]
fn cursor_to_view_middle(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursor_to_view_zone(Zone::Middle) {
        run(editor, id, Hook::CursorMoved)
    }
}

#[action("Cursors: Goto to bottom of view")]
fn cursor_to_view_bottom(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursor_to_view_zone(Zone::Bottom) {
        run(editor, id, Hook::CursorMoved)
    }
}
