use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{
    find_range,
    movement::{self, next_line_start},
    paragraph_at_pos, word_at_pos, BufferRange, Cursor, Range, Searcher,
};

use crate::editor::{
    buffers::Buffer,
    hooks::Hook,
    windows::{Cursors, Focus, HistoryKind, Prompt, Window, Zone},
    Editor,
};

use sanedit_server::ClientId;

use super::{
    hooks,
    window::{focus, mode_select},
    ActionResult,
};

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

    {
        let mut cursors = win.cursors.cursors_mut();
        for cursor in cursors.iter_mut() {
            let pos = cursor.pos();
            let range = (f)(&slice, pos);

            if let Some((range, col)) = range {
                if !range.is_empty() {
                    cursor.select(range);
                    cursor.set_column(col);
                    changed = true;
                }
            }
        }
    }

    if changed {
        win.view_to_cursor(buf);
        mode_select(editor, id);
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

    for cursor in win.cursors.cursors_mut().iter_mut() {
        let pos = cursor.pos();
        let range = (f)(&slice, pos);

        if let Some(range) = range {
            if !range.is_empty() {
                cursor.select(range);
                changed = true;
            }
        }
    }

    if changed {
        win.view_to_around_cursor_zone(buf, Zone::Middle);
        mode_select(editor, id);
        hooks::run(editor, id, Hook::CursorMoved);
        ActionResult::Ok
    } else {
        ActionResult::Skipped
    }
}

#[action("Select: Line")]
fn select_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_with_col(editor, id, |slice, pos| {
        let start = movement::start_of_line(slice, pos);
        let end = next_line_start(slice, pos);
        if start == end {
            None
        } else {
            Some((Range::from(start..end), 0))
        }
    })
}

#[action("Select: Line content")]
fn select_line_content(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_with_col(editor, id, |slice, pos| {
        let start = movement::first_char_of_line(slice, pos);
        let end = movement::end_of_line(slice, pos);
        if start == end {
            None
        } else {
            Some((Range::from(start..end), 0))
        }
    })
}

#[action("Select: Line without end of line")]
fn select_line_without_eol(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_with_col(editor, id, |slice, pos| {
        let start = movement::start_of_line(slice, pos);
        let end = movement::end_of_line(slice, pos);
        if start == end {
            None
        } else {
            Some((Range::from(start..end), 0))
        }
    })
}

#[action("Select: Buffer")]
fn select_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    select(editor, id, |slice, _| Some(Range::from(0..slice.len())))
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
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Select pattern")
        .simple()
        .history(HistoryKind::Search)
        .on_confirm(move |editor, id, out| {
            let (win, buf) = editor.win_buf_mut(id);
            let pattern = getf!(out.text());
            let searcher = getf!(get_pattern_searcher(pattern, win));
            let selections = get_cursor_selections(win, buf);

            let mut cursors = vec![];
            for range in selections {
                let slice = buf.slice(range);
                for mat in searcher.find_iter(&slice) {
                    let mut sel = mat.range();
                    sel.forward(slice.start());
                    let cursor = Cursor::new_select(sel);
                    cursors.push(cursor);
                }
            }

            if cursors.is_empty() {
                return ActionResult::Failed;
            }

            win.cursors = Cursors::from(cursors);
            win.view_to_around_cursor_zone(buf, Zone::Middle);
            mode_select(editor, id);
            ActionResult::Ok
        })
        .build();

    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

fn get_cursor_selections(win: &Window, buf: &Buffer) -> Vec<BufferRange> {
    let mut ranges: Vec<BufferRange> = vec![];
    for cursor in win.cursors().iter() {
        if let Some(sel) = cursor.selection() {
            ranges.push(sel);
        }
    }

    // If no cursor selections select the whole buffer
    if ranges.is_empty() {
        ranges.push(Range::from(0..buf.len()));
    }

    ranges
}

fn get_pattern_searcher(pattern: &str, win: &mut Window) -> Option<Searcher> {
    if pattern.is_empty() {
        // If empty pattern try last search
        let search = &win.search.current;
        if search.pattern.is_empty() {
            win.warn_msg("No pattern found");
            return None;
        }
        let searcher = Searcher::with_options(&search.pattern, &search.opts);
        if searcher.is_err() {
            win.warn_msg("Invalid pattern");
            return None;
        }

        searcher.unwrap().into()
    } else {
        let searcher = Searcher::new(pattern);
        if searcher.is_err() {
            win.warn_msg("Invalid pattern");
            return None;
        }

        searcher.unwrap().0.into()
    }
}
