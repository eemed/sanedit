use std::mem;

use sanedit_core::{at_start_of_line, is_indent_at_pos};

use crate::editor::{
    buffers::BufferError,
    hooks::Hook,
    windows::{Focus, Prompt},
    Editor,
};

use sanedit_server::ClientId;

use super::{
    completion, hooks::run, movement::{end_of_line, prev_line}, window::focus, ActionResult
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
            let path = get!(out.path());
            let (_win, buf) = editor.win_buf_mut(id);
            buf.set_path(path);
            save.execute(editor, id);
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

    let hook = Hook::BufChanged(buf.id);
    run(editor, id, hook);

    ActionResult::Ok
}

#[action("Buffer: Insert tab")]
fn insert_tab(editor: &mut Editor, id: ClientId) -> ActionResult {
    run(editor, id, Hook::InsertPre);

    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);
    let primary = win.cursors.primary().pos();

    if win.cursors().has_selections() {
        if win.indent_cursor_lines(buf).is_ok() {
            let hook = Hook::BufChanged(buf.id);
            run(editor, id, hook);
        }
        ActionResult::Ok
    } else if win.cursors.len() == 1
        && !is_indent_at_pos(&slice, primary)
        && !at_start_of_line(&slice, primary)
    {
        // If single cursor not in indentation try completion
        completion::complete.execute(editor, id)
    } else if win.indent(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Skipped
    }
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
    // Disable autopair for this action
    let (win, _buf) = editor.win_buf_mut(id);
    let restore = mem::replace(&mut win.config.autopair, false);

    prev_line.execute(editor, id);
    end_of_line.execute(editor, id);
    insert_newline.execute(editor, id);

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

    ActionResult::Ok
}

#[action("Buffer: Comment lines")]
fn comment_lines(editor: &mut Editor, id: ClientId) -> ActionResult{
    let (win, buf) = win_buf!(editor, id);
    let Some(ft) = &buf.filetype else {
        win.warn_msg("No filetype set");
        return ActionResult::Skipped;
    };
    let Some(ftconfig) = editor.filetype_config.get(ft) else {
        win.warn_msg("No comment string set for filetype");
        return ActionResult::Skipped;
    };
    let comment = &ftconfig.general.comment;
    if comment.is_empty() {
        win.warn_msg("No comment string set for filetype");
        return ActionResult::Skipped;
    }

    if win.comment_cursor_lines(buf, comment).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    ActionResult::Ok
}

#[action("Buffer: Uncomment lines")]
fn uncomment_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let Some(ft) = &buf.filetype else {
        win.warn_msg("No filetype set");
        return ActionResult::Skipped;
    };
    let Some(ftconfig) = editor.filetype_config.get(ft) else {
        win.warn_msg("No comment string set for filetype");
        return ActionResult::Skipped;
    };
    let comment = &ftconfig.general.comment;
    if comment.is_empty() {
        win.warn_msg("No comment string set for filetype");
        return ActionResult::Skipped;
    }

    if win.uncomment_cursor_lines(buf, comment).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    ActionResult::Ok
}

#[action("Buffer: Toggle comment lines")]
fn toggle_comment_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let Some(ft) = &buf.filetype else {
        win.warn_msg("No filetype set");
        return ActionResult::Skipped;
    };
    let Some(ftconfig) = editor.filetype_config.get(ft) else {
        win.warn_msg("No comment string set for filetype");
        return ActionResult::Skipped;
    };
    let comment = &ftconfig.general.comment;
    if comment.is_empty() {
        win.warn_msg("No comment string set for filetype");
        return ActionResult::Skipped;
    }

    if win.toggle_comment_cursor_lines(buf, comment).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }

    ActionResult::Ok
}
