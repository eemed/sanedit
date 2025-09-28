use sanedit_core::{
    movement::{end_of_line, start_of_line},
    word_at_pos, BufferRange, Cursor,
};
use sanedit_messages::{key, redraw::Point, MouseEvent};
use sanedit_server::ClientId;

use crate::{
    common::window::pos_at_point,
    editor::{hooks::Hook, windows::MouseClick, Editor},
};

use super::{
    hooks,
    window::{mode_normal, mode_select},
    ActionResult,
};

pub(crate) fn on_button_down_left_click(
    editor: &mut Editor,
    id: ClientId,
    event: MouseEvent,
) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.mouse.on_click(event.point);

    if event.mods & key::CONTROL != 0 {
        new_to_point(editor, id, event.point);
    } else if event.mods == 0 {
        match win.mouse.clicks() {
            MouseClick::Single => goto_position(editor, id, event.point),
            MouseClick::Double => select_word(editor, id, event.point),
            MouseClick::Triple => select_line(editor, id, event.point),
        };
    }

    ActionResult::Ok
}

pub(crate) fn on_drag(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    match win.mouse.clicks() {
        MouseClick::Single => drag_normal(editor, id, point),
        MouseClick::Double => drag_word(editor, id, point),
        MouseClick::Triple => drag_line(editor, id, point),
    };

    ActionResult::Ok
}

fn drag_normal(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.cursors.cursors_mut().remove_except_primary();
    let pos = getf!(pos_at_point(win, point));

    {
        let mut cursors = win.cursors.cursors_mut();
        let primary = cursors.primary();
        if !primary.is_selecting() {
            let ppos = primary.pos();
            if ppos < pos {
                primary.select(ppos..pos);
            } else {
                primary.select(pos..ppos);
                primary.swap_selection_dir();
            }
        } else {
            primary.goto(pos);
        }
    }
    hooks::run(editor, id, Hook::CursorMoved);
    mode_select(editor, id);

    ActionResult::Ok
}

fn drag_word(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.cursors_mut().remove_except_primary();
    let slice = buf.slice(..);
    let pos = getf!(pos_at_point(win, point));
    let word = getf!(word_at_pos(&slice, pos));
    drag_impl(editor, id, word)
}

fn drag_line(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors.cursors_mut().remove_except_primary();
    let slice = buf.slice(..);
    let pos = getf!(pos_at_point(win, point));
    let sol = start_of_line(&slice, pos);
    let eol = end_of_line(&slice, pos);
    drag_impl(editor, id, (sol..eol).into())
}

fn drag_impl(editor: &mut Editor, id: ClientId, range: BufferRange) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    {
        let mut cursors = win.cursors.cursors_mut();
        cursors.remove_except_primary();
        let primary = cursors.primary();

        let sel = getf!(primary.selection());

        if sel.includes(range) && (sel.start != range.start || sel.end != range.end) {
            if primary.pos() == sel.start {
                primary.goto(range.start);
            } else {
                primary.goto(range.end);
            }
        } else {
            if range.end <= sel.start && primary.pos() != sel.start
                || range.start >= sel.end && primary.pos() != sel.end
            {
                primary.swap_selection_dir();
            }

            primary.extend_to_include(range);
        }
    }

    hooks::run(editor, id, Hook::CursorMoved);
    mode_select(editor, id);
    ActionResult::Ok
}

fn new_to_point(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    let pos = getf!(pos_at_point(win, point));
    win.cursors.cursors_mut().push(Cursor::new(pos));
    ActionResult::Ok
}

fn goto_position(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    let pos = getf!(pos_at_point(win, point));
    {
        let mut cursors = win.cursors.cursors_mut();
        cursors.remove_except_primary();
        let primary = cursors.primary();
        primary.stop_selection();
        primary.goto(pos);
    }
    hooks::run(editor, id, Hook::CursorMoved);
    mode_normal(editor, id);

    ActionResult::Ok
}

fn select_word(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);
    let pos = getf!(pos_at_point(win, point));
    let word = getf!(word_at_pos(&slice, pos));

    {
        let mut cursors = win.cursors.cursors_mut();
        cursors.remove_except_primary();
        let primary = cursors.primary();
        primary.stop_selection();
        primary.select(word);
    }
    hooks::run(editor, id, Hook::CursorMoved);
    mode_select(editor, id);

    ActionResult::Ok
}

fn select_line(editor: &mut Editor, id: ClientId, point: Point) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);
    let pos = getf!(pos_at_point(win, point));
    let sol = start_of_line(&slice, pos);
    let eol = end_of_line(&slice, pos);

    {
        let mut cursors = win.cursors.cursors_mut();
        cursors.remove_except_primary();
        let primary = cursors.primary();
        primary.stop_selection();
        primary.select(sol..eol);
    }
    hooks::run(editor, id, Hook::CursorMoved);
    mode_select(editor, id);
    ActionResult::Ok
}
