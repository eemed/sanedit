use std::{mem, sync::Arc};

use sanedit_core::{at_start_of_line, is_indent_at_pos};

use crate::{
    actions::movement::start_of_buffer,
    editor::{
        buffers::{Buffer, BufferError},
        hooks::Hook,
        language::Languages,
        windows::{Focus, NextKeyFunction, Prompt, Window},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{
    completion,
    cursors::{remove_cursor_selections, swap_selection_dir},
    hooks::run,
    movement::{end_of_line, prev_line},
    snippets::snippet_jump_next,
    text_objects::select_line,
    window::{focus, mode_insert, mode_normal},
    ActionResult,
};

#[action("Buffer: Remove character after cursor")]
fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) -> ActionResult {
    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    if win.remove_grapheme_after_cursors(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    ActionResult::Ok
}

#[action("Buffer: Remove character before cursor")]
fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) -> ActionResult {
    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    if win.remove_grapheme_before_cursors(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    ActionResult::Ok
}

#[action("Buffer: Undo")]
pub(crate) fn undo(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.undo(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}

#[action("Buffer: Redo")]
pub(crate) fn redo(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.redo(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        run(editor, id, Hook::CursorMoved);
    }

    ActionResult::Ok
}

pub(crate) fn insert(editor: &mut Editor, id: ClientId, text: &str) {
    let (win, _buf) = editor.win_buf_mut(id);

    use Focus::*;
    match win.focus() {
        Search | Prompt => {
            win.prompt.insert_at_cursor(text);
            if let Some(on_input) = win.prompt.on_input() {
                let input = win.prompt.input().to_string();
                (on_input)(editor, id, &input)
            }
        }
        Completion | Window => {
            run(editor, id, Hook::InsertPre);
            let (win, buf) = editor.win_buf_mut(id);
            if win.insert_at_cursors(buf, text).is_ok() {
                win.view_to_cursor(buf);
                let hook = Hook::BufChanged(buf.id);
                run(editor, id, hook);
            }
        }
        Filetree => {}
        Locations => {}
    }
}

#[action("Buffer: Save")]
fn save(editor: &mut Editor, id: ClientId) -> ActionResult {
    run(editor, id, Hook::BufSavedPre);
    let (win, buf) = editor.win_buf_mut(id);

    match win.save_buffer(buf) {
        Ok(()) => {
            run(editor, id, Hook::BufSavedPost);
            ActionResult::Ok
        }
        Err(e) => {
            if let Some(BufferError::NoSavePath) = e.root_cause().downcast_ref::<BufferError>() {
                // Clear error message, as we execute a new fix action
                win.clear_msg();
                save_as.execute(editor, id)
            } else {
                ActionResult::Failed
            }
        }
    }

    // let big_th = editor.options.big_file_threshold_bytes;
    // let (win, buf) = editor.win_buf_mut(id);
    // let size = buf.len() as u64;
    // let is_big = size >= big_th;
    // if is_big {
    //     todo!()
    //     // buf.read_only = true;
    //     // let job = jobs::Save::new(id, ropt);
    //     // editor.job_broker.request(job);
    // } else {
    // }
}

#[action("Buffer: Save as")]
fn save_as(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Save as")
        .simple()
        .on_confirm(|editor, id, out| {
            let path = getf!(out.path());
            let (_win, buf) = editor.win_buf_mut(id);
            buf.set_path(path);
            save.execute(editor, id)
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Buffer: Insert newline")]
fn insert_newline(editor: &mut Editor, id: ClientId) -> ActionResult {
    run(editor, id, Hook::InsertPre);
    let (win, buf) = editor.win_buf_mut(id);
    let _ = win.insert_newline(buf);
    win.view_to_cursor(buf);

    let hook = Hook::BufChanged(buf.id);
    run(editor, id, hook);

    ActionResult::Ok
}

#[action("Buffer: Insert tab")]
fn insert_tab(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);

    // Snippet
    if !win.snippets.is_empty() {
        snippet_jump_next.execute(editor, id);
        return ActionResult::Ok;
    }

    let slice = buf.slice(..);
    let primary = win.cursors.primary().pos();

    if win.cursors().has_selections() {
        run(editor, id, Hook::InsertPre);
        let (win, buf) = editor.win_buf_mut(id);
        if win.indent_cursor_lines(buf).is_ok() {
            let hook = Hook::BufChanged(buf.id);
            run(editor, id, hook);
        }
        return ActionResult::Ok;
    } else if win.cursors.len() == 1
        && !is_indent_at_pos(&slice, primary)
        && !at_start_of_line(&slice, primary)
    {
        // If single cursor not in indentation try completion
        return completion::complete.execute(editor, id);
    } else {
        run(editor, id, Hook::InsertPre);
        let (win, buf) = editor.win_buf_mut(id);
        if win.indent(buf).is_ok() {
            let hook = Hook::BufChanged(buf.id);
            run(editor, id, hook);
            return ActionResult::Ok;
        }
    }

    ActionResult::Skipped
}

#[action("Buffer: Dedent")]
fn backtab(editor: &mut Editor, id: ClientId) -> ActionResult {
    run(editor, id, Hook::InsertPre);
    let (win, buf) = editor.win_buf_mut(id);
    if win.dedent_cursor_lines(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    ActionResult::Ok
}

#[action("Buffer: Remove to line end")]
fn remove_to_end_of_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.remove_line_after_cursor(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }
    ActionResult::Ok
}

#[action("Buffer: Remove trailing whitespace")]
fn strip_trailing_whitespace(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.strip_trailing_whitespace(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }
    ActionResult::Ok
}

#[action("Buffer: Newline below")]
fn newline_below(editor: &mut Editor, id: ClientId) -> ActionResult {
    mode_insert(editor, id);

    // Disable autopair for this action
    let (win, _buf) = editor.win_buf_mut(id);
    let restore = mem::replace(&mut win.config.autopair, false);

    end_of_line.execute(editor, id);
    insert_newline.execute(editor, id);

    let (win, _buf) = editor.win_buf_mut(id);
    win.config.autopair = restore;
    ActionResult::Ok
}

#[action("Buffer: Newline above")]
fn newline_above(editor: &mut Editor, id: ClientId) -> ActionResult {
    mode_insert(editor, id);
    // Disable autopair for this action
    let (win, _buf) = editor.win_buf_mut(id);
    let restore = mem::replace(&mut win.config.autopair, false);

    let (win, _buf) = editor.win_buf_mut(id);
    let orig = win.cursors().primary().pos();
    prev_line.execute(editor, id);
    let (win, _buf) = editor.win_buf_mut(id);
    let prev = win.cursors().primary().pos();
    let on_first_line = orig == prev;
    if !on_first_line {
        end_of_line.execute(editor, id);
    } else {
        start_of_buffer.execute(editor, id);
    }

    insert_newline.execute(editor, id);

    if on_first_line {
        prev_line.execute(editor, id);
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.config.autopair = restore;
    ActionResult::Ok
}

#[action("Buffer: Align cursor columns")]
fn align_cursor_columns(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.align_cursors(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    mode_normal(editor, id);
    ActionResult::Ok
}

fn get_comment<'a>(
    languages: &'a Languages,
    win: &mut Window,
    buf: &Buffer,
    show_error: bool,
) -> Option<&'a str> {
    let Some(ft) = &buf.language else {
        if show_error {
            win.warn_msg("No language set");
        }
        return None;
    };
    let Some(langconfig) = languages.get(&ft) else {
        if show_error {
            win.warn_msg("No comment string set for language");
        }
        return None;
    };
    let comment = &langconfig.comment;
    if comment.is_empty() {
        if show_error {
            win.warn_msg("No comment string set for language");
        }
        return None;
    }

    Some(comment)
}

#[action("Buffer: Comment lines")]
fn comment_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    match get_comment(&editor.languages, win, buf, true) {
        Some(comment) => {
            if win.comment_cursor_lines(buf, comment).is_ok() {
                let hook = Hook::BufChanged(buf.id);
                run(editor, id, hook);
            }
            mode_normal(editor, id);
            ActionResult::Ok
        }
        None => ActionResult::Skipped,
    }
}

#[action("Buffer: Uncomment lines")]
fn uncomment_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    match get_comment(&editor.languages, win, buf, true) {
        Some(comment) => {
            if win.uncomment_cursor_lines(buf, comment).is_ok() {
                let hook = Hook::BufChanged(buf.id);
                run(editor, id, hook);
            }
            mode_normal(editor, id);
            ActionResult::Ok
        }
        None => ActionResult::Skipped,
    }
}

#[action("Buffer: Toggle comment lines")]
fn toggle_comment_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    match get_comment(&editor.languages, win, buf, true) {
        Some(comment) => {
            if win.toggle_comment_cursor_lines(buf, comment).is_ok() {
                let hook = Hook::BufChanged(buf.id);
                run(editor, id, hook);
            }
            mode_normal(editor, id);
            ActionResult::Ok
        }
        None => ActionResult::Skipped,
    }
}

#[action("Buffer: Join lines")]
fn join_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let comment = get_comment(&editor.languages, win, buf, false).unwrap_or("");

    if win.join_lines(buf, comment).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    mode_normal(editor, id);
    ActionResult::Ok
}

#[action("Buffer: Insert literal")]
fn insert_literal(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.next_key_handler = Some(NextKeyFunction(Arc::new(|editor, id, key| {
        let input = key.literal_utf8();
        insert(editor, id, &input);
        ActionResult::Ok
    })));
    ActionResult::Ok
}

#[action("Buffer: Remove line")]
fn remove_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_line.execute(editor, id);
    remove_cursor_selections.execute(editor, id)
}

#[action("Buffer: Remove to line end")]
fn remove_to_eol(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.cursors.has_selections() {
        return ActionResult::Skipped;
    }

    win.cursors.start_selection();
    end_of_line.execute(editor, id);
    swap_selection_dir.execute(editor, id);
    remove_cursor_selections.execute(editor, id)
}
