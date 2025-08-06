use sanedit_buffer::MarkResult;
use sanedit_messages::redraw::Point;
use sanedit_utils::ring::Ref;

use crate::{
    common::window::pos_at_point,
    editor::{
        buffers::{BufferId, Buffers},
        hooks::Hook,
        windows::{Cursors, Jump, JumpGroup, Jumps, Window, Zone},
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
    movement::{self, next_grapheme},
    window::{mode_insert, mode_normal, mode_select},
    ActionResult,
};

#[action("Cursors: Select next word")]
fn select_to_next_word(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        if !cursor.is_selecting() {
            cursor.start_selection();
        }
    }
    movement::next_word_end.execute(editor, id)
}

#[action("Cursors: Select previous word")]
fn select_to_prev_word(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    for cursor in win.cursors.cursors_mut() {
        if !cursor.is_selecting() {
            cursor.start_selection();
        }
    }
    movement::prev_word_start.execute(editor, id)
}

#[action("Cursors: New on next line")]
fn new_cursor_to_next_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = next_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
    ActionResult::Ok
}

#[action("Cursors: New on previous line")]
fn new_cursor_to_prev_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary();
    let (pos, _col) = prev_line(&buf.slice(..), cursor, win.display_options());
    win.cursors.push_primary(Cursor::new(pos));
    ActionResult::Ok
}

#[action("Cursors: New on next search match")]
fn new_cursor_to_next_search_match(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let last_search = &win.search.current;
    let ppos = win.cursors.primary().pos();

    let Ok(searcher) = Searcher::with_options(&last_search.pattern, &last_search.opts) else {
        return ActionResult::Failed;
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

        mode_select(editor, id);
    }

    ActionResult::Ok
}

#[action("Cursors: New on each search match")]
fn new_cursor_to_all_search_matches(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    // win.cursors.remove_secondary_cursors();

    let last_search = &win.search.current;

    let Ok(searcher) = Searcher::with_options(&last_search.pattern, &last_search.opts) else {
        return ActionResult::Failed;
    };
    let slice = buf.slice(..);

    let cursors: Vec<Cursor> = searcher
        .find_iter(&slice)
        .map(|mat| {
            let range = mat.range();
            Cursor::new_select(&range)
        })
        .collect();

    if cursors.is_empty() {
        return ActionResult::Skipped;
    }

    win.cursors = Cursors::from(cursors);
    mode_select(editor, id);

    ActionResult::Ok
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
fn start_selection(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.start_selection();
    mode_select(editor, id);
    next_grapheme.execute(editor, id);
    ActionResult::Ok
}

#[action("Cursors: Cancel selection")]
fn stop_selection(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.stop_selection();
    mode_normal(editor, id);
    ActionResult::Ok
}

#[action("Cursors: Remove secondary cursors")]
fn keep_only_primary(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    if win.cursors.cursors().iter().any(|c| c.is_selecting()) {
        win.cursors.stop_selection();
    } else {
        win.cursors.remove_except_primary();

        let (win, _buf) = editor.win_buf_mut(id);
        win.cursors.primary_mut().stop_selection();
    }

    ActionResult::Ok
}

#[action("Cursors: Swap selection direction")]
fn swap_selection_dir(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.swap_selection_dir();

    ActionResult::Ok
}

#[action("Cursors: Remove primary cursor")]
fn remove_primary_cursor(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.remove_primary();

    ActionResult::Ok
}

#[action("Cursors: Make next cursor primary")]
fn make_next_cursor_primary(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.primary_next();
    win.view_to_around_cursor_zone(buf, Zone::Middle);
    ActionResult::Ok
}

#[action("Cursors: Make previous cursor primary")]
fn make_prev_cursor_primary(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.primary_prev();
    win.view_to_around_cursor_zone(buf, Zone::Middle);
    ActionResult::Ok
}

#[action("Cursors: Merge overlapping")]
fn merge_overlapping_cursors(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.merge_overlapping();
    ActionResult::Ok
}

#[action("Cursors: Remove selections")]
fn remove_cursor_selections(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.config.editor.copy_on_delete {
        editor.copy_to_clipboard(id);
    }

    let (win, buf) = editor.win_buf_mut(id);
    let res = match win.remove_cursor_selections(buf) {
        Ok(true) => {
            win.view_to_around_cursor_zone(buf, Zone::Middle);
            let hook = Hook::BufChanged(buf.id);
            run(editor, id, hook);
            ActionResult::Ok
        }
        _ => ActionResult::Skipped,
    };

    mode_normal(editor, id);
    res
}

#[action("Cursors: Change cursor selections")]
fn change_cursor_selections(editor: &mut Editor, id: ClientId) -> ActionResult {
    remove_cursor_selections.execute(editor, id);
    mode_insert(editor, id);
    ActionResult::Ok
}

#[action("Cursors: New cursors on line starts and goto insert mode")]
fn cursors_to_lines_start(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors_to_lines_start(buf);
    mode_insert(editor, id);
    hooks::run(editor, id, Hook::CursorMoved);
    ActionResult::Ok
}

#[action("Cursors: New cursors on line ends and goto insert mode")]
fn cursors_to_lines_end(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors_to_lines_end(buf);
    mode_insert(editor, id);
    hooks::run(editor, id, Hook::CursorMoved);
    ActionResult::Ok
}

#[action("Cursors: Goto to previous change")]
fn jump_prev_change(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.cursors_to_prev_change(buf) {
        run(editor, id, Hook::CursorMoved)
    }
    ActionResult::Ok
}

#[action("Cursors: Goto to next change")]
fn jump_next_change(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.cursors_to_next_change(buf) {
        run(editor, id, Hook::CursorMoved)
    }
    ActionResult::Ok
}

fn find_prev_jump(win: &Window, buffers: &Buffers, original_bid: BufferId) -> Option<Ref> {
    // for example iter backwards
    // until mark is Found
    // or
    //  buf changes => iter until mark is Found
    //      or
    //      if not goto Deleted marker once
    //
    // This would go to previous buffers even if mark is not found
    // but skip all other deleted markers
    //

    // If curent position was a jump group, used to skip over jump that would do
    // nothing
    let current = {
        let primary = win.cursors.primary().pos();
        let mark = buffers.get(win.buffer_id()).unwrap().mark(primary);
        let jump = Jump::new(mark, None);
        JumpGroup::new(win.buffer_id(), vec![jump])
    };

    let cursor_jumps = &win.cursor_jumps;
    // Take previous or last if none selected
    let mut item = {
        match cursor_jumps.current() {
            Some((cursor, _)) => cursor_jumps.prev_of_ref(&cursor),
            None => cursor_jumps.last(),
        }
    };
    let mut previous: Option<(Ref, BufferId)> = None;

    // Skip to if this is current position
    if let Some((cursor, group)) = &item {
        if *group == &current {
            item = cursor_jumps.prev_of_ref(&cursor);
        }
    }

    while let Some((cursor, group)) = item {
        let gbid = group.buffer_id();

        // Return if marks are found
        if let Some(buf) = buffers.get(gbid) {
            let found = group.jumps().iter().all(|jump| {
                let mark = buf.mark_to_pos(jump.start());
                matches!(mark, MarkResult::Found(_))
            });

            if found {
                return Some(cursor);
            }
        }

        if let Some((pcursor, pbid)) = &previous {
            // We have already looped to next buffer
            // and current entry is also in another buffer
            if *pbid != original_bid && *pbid != gbid {
                return Some(pcursor.clone());
            }
        }

        // Goto previous element and record current
        item = cursor_jumps.prev_of_ref(&cursor);
        previous = Some((cursor, gbid));
    }

    // If buffer changed goto previous buffer
    if let Some((pcursor, pbid)) = &previous {
        if *pbid != original_bid {
            return Some(pcursor.clone());
        }
    }

    None
}

fn find_next_jump<const N: usize>(
    cursor_jumps: &Jumps<N>,
    buffers: &Buffers,
    original_bid: BufferId,
) -> Option<Ref> {
    // for example iter backwards
    // until mark is Found
    // or
    //  buf changes => iter until mark is Found
    //      or
    //      if not goto Deleted marker once
    //
    // This would go to previous buffers even if mark is not found
    // but skip all other deleted markers

    let (mut cursor, _) = cursor_jumps.current()?;
    let mut previous: Option<(Ref, BufferId)> = None;

    while let Some((gcursor, group)) = cursor_jumps.next_of_ref(&cursor) {
        cursor = gcursor;

        let gbid = group.buffer_id();

        // Return if marks are found
        if let Some(buf) = buffers.get(gbid) {
            let found = group.jumps().iter().all(|jump| {
                let mark = buf.mark_to_pos(jump.start());
                matches!(mark, MarkResult::Found(_))
            });

            if found {
                return Some(cursor);
            }
        }

        if let Some((pcursor, pbid)) = &previous {
            // We have went to next buffer
            // and current entry is also in another buffer
            if *pbid != original_bid && *pbid != gbid {
                return Some(pcursor.clone());
            }
        }

        previous = Some((cursor.clone(), gbid));
    }

    if let Some((pcursor, pbid)) = &previous {
        if *pbid != original_bid {
            return Some(pcursor.clone());
        }
    }

    None
}

#[action("Cursors: Goto previous jump")]
fn jump_prev(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let bid = buf.id;
    let cursor = getf!(find_prev_jump(&win, &editor.buffers, bid));

    // Add position if jumping backwards
    let (win, buf) = win_buf!(editor, id);
    if win.cursor_jumps.current().is_none() {
        let primary = win.cursors.primary().pos();
        let mark = buf.mark(primary);
        let jump = Jump::new(mark, None);
        let group = JumpGroup::new(win.buffer_id(), vec![jump]);
        let is_diff = win
            .cursor_jumps
            .last()
            .map(|(_, g)| &group != g)
            .unwrap_or(true);
        if is_diff {
            win.push_new_cursor_jump(buf);
        }
    }

    jump_to_ref(editor, id, cursor)
}

#[action("Cursors: Goto next jump")]
fn jump_next(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let bid = buf.id;
    let cursor = getf!(find_next_jump(&win.cursor_jumps, &editor.buffers, bid));
    jump_to_ref(editor, id, cursor)
}

/// Jump to reference
pub(crate) fn jump_to_ref(editor: &mut Editor, id: ClientId, cursor: Ref) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let bid = buf.id;
    let group = getf!(win.cursor_jumps.get(&cursor));
    let next_bid = group.buffer_id();

    if next_bid != bid {
        run(editor, id, Hook::BufLeave(bid));
    }

    let (win, _buf) = win_buf!(editor, id);
    let buf = getf!(editor.buffers.get(next_bid));
    win.goto_cursor_jump(cursor, buf);
    if next_bid != bid {
        run(editor, id, Hook::BufEnter(next_bid));
    }
    run(editor, id, Hook::CursorMoved);
    ActionResult::Ok
}

#[action("Cursors: Goto to top of view")]
fn cursor_to_view_top(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursor_to_view_zone(Zone::Top) {
        run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}

#[action("Cursors: Goto to middle of view")]
fn cursor_to_view_middle(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursor_to_view_zone(Zone::Middle) {
        run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}

#[action("Cursors: Goto to bottom of view")]
fn cursor_to_view_bottom(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursor_to_view_zone(Zone::Bottom) {
        run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}
