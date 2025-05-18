use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{
    find_range,
    movement::{self, next_line_start},
    paragraph_at_pos, word_at_pos, BufferRange, Range,
};

use crate::editor::{hooks::Hook, Editor};

use sanedit_server::ClientId;

use super::{hooks, ActionResult};

fn select_range(
    editor: &mut Editor,
    id: ClientId,
    start: &str,
    end: &str,
    include: bool,
) -> ActionResult {
    select(editor, id, |slice, pos| {
        find_range(slice, pos, start, end, include)
    })
}

fn select_with_col<F: Fn(&PieceTreeSlice, u64) -> Option<(BufferRange, usize)>>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
) -> ActionResult {
    let mut changed = false;
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);

    for cursor in win.cursors.cursors_mut() {
        let pos = cursor.pos();
        let range = (f)(&slice, pos);

        if let Some((range, col)) = range {
            if !range.is_empty() {
                cursor.select(&range);
                cursor.set_column(col);
                changed = true;
            }
        }
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
        ActionResult::Ok
    } else {
        ActionResult::Skipped
    }
}

fn select<F: Fn(&PieceTreeSlice, u64) -> Option<BufferRange>>(
    editor: &mut Editor,
    id: ClientId,
    f: F,
) -> ActionResult {
    let mut changed = false;
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);

    for cursor in win.cursors.cursors_mut() {
        let pos = cursor.pos();
        let range = (f)(&slice, pos);

        if let Some(range) = range {
            if !range.is_empty() {
                cursor.select(&range);
                changed = true;
            }
        }
    }

    if changed {
        win.view_to_cursor(buf);
        hooks::run(editor, id, Hook::CursorMoved);
        ActionResult::Ok
    } else {
        ActionResult::Skipped
    }
}

#[action("Select: Line")]
fn select_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_with_col(editor, id, |slice, pos| {
        let start = movement::start_of_line(&slice, pos);
        let end = next_line_start(&slice, pos);
        if start == end {
            None
        } else {
            Some((Range::new(start, end), 0))
        }
    })
}

#[action("Select: In curly brackets")]
fn select_curly(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "{", "}", false)
}

#[action("Select: In curly brackets (incl)")]
fn select_curly_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "{", "}", true)
}

#[action("Select: In parentheses")]
fn select_parens(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "(", ")", false)
}

#[action("Select: In parentheses (incl)")]
fn select_parens_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "(", ")", true)
}

#[action("Select: In square brackets")]
fn select_square(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "[", "]", false)
}

#[action("Select: In square brackets (incl)")]
fn select_square_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "[", "]", true)
}

#[action("Select: In angle brackets")]
fn select_angle(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "<", ">", false)
}

#[action("Select: In angle brackets (incl)")]
fn select_angle_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "<", ">", true)
}

#[action("Select: In single quotes (incl)")]
fn select_single_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "'", "'", true)
}

#[action("Select: In single quotes")]
fn select_single(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "'", "'", false)
}

#[action("Select: In double quotes (incl)")]
fn select_double_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "\"", "\"", true)
}

#[action("Select: In double quotes")]
fn select_double(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "\"", "\"", false)
}

#[action("Select: In backticks (incl)")]
fn select_backtick_incl(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "`", "`", true)
}

#[action("Select: In backticks")]
fn select_backtick(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_range(editor, id, "`", "`", false)
}

#[action("Select: Word")]
fn select_word(editor: &mut Editor, id: ClientId) -> ActionResult {
    select(editor, id, word_at_pos)
}

#[action("Select: Paragraph")]
fn select_paragraph(editor: &mut Editor, id: ClientId) -> ActionResult {
    select(editor, id, paragraph_at_pos)
}

#[action("Select: Pattern")]
fn select_pattern(editor: &mut Editor, id: ClientId) -> ActionResult {
    // TODO
    ActionResult::Ok
}
