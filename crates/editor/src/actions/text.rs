use std::path::PathBuf;

use crate::{
    common::indent::indent_at_pos,
    editor::{
        buffers::BufferError,
        hooks::Hook,
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

use super::{completion, hooks::run, jobs};

#[action("Remove character after cursor")]
fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    let _ = win.remove_grapheme_after_cursors(buf);
    run(editor, id, Hook::BufChanged);
}

#[action("Remove character before cursor")]
fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::RemovePre);
    let (win, buf) = editor.win_buf_mut(id);
    let _ = win.remove_grapheme_before_cursors(buf);
    run(editor, id, Hook::BufChanged);
}

#[action("Undo a change")]
pub(crate) fn undo(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if win.undo(buf).is_ok() {
        run(editor, id, Hook::BufChanged);
        run(editor, id, Hook::CursorMoved);
    }
}

#[action("Redo a change")]
pub(crate) fn redo(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if win.redo(buf).is_ok() {
        run(editor, id, Hook::BufChanged);
        run(editor, id, Hook::CursorMoved);
    }
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
            let _ = win.insert_at_cursors(buf, text);
            run(editor, id, Hook::BufChanged);
        }
        Filetree => {}
        Locations => {}
    }
}

#[action("Save file")]
fn save(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);

    if let Err(e) = win.save_buffer(buf) {
        if let Some(BufferError::NoSavePath) = e.root_cause().downcast_ref::<BufferError>() {
            // Clear error message, as we execute a new fix action
            win.clear_msg();
            save_as.execute(editor, id)
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

#[action("Save as")]
fn save_as(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Save as")
        .simple()
        .on_confirm(|editor, id, path| {
            let (_win, buf) = editor.win_buf_mut(id);
            buf.set_path(PathBuf::from(path));
            save.execute(editor, id);
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Insert a newline to each cursor")]
fn insert_newline(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::InsertPre);
    let (win, buf) = editor.win_buf_mut(id);
    let _ = win.insert_newline(buf);
    run(editor, id, Hook::BufChanged);
}

#[action("Insert a tab to each cursor")]
fn insert_tab(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::InsertPre);

    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);
    if win.cursors().has_selections() {
        win.indent_cursor_lines(buf);
    } else if win.cursors.len() == 1 && indent_at_pos(&slice, win.cursors.primary().pos()).is_none()
    {
        // If single cursor not in indentation try completion
        completion::complete.execute(editor, id);
    } else {
        let (win, buf) = editor.win_buf_mut(id);
        win.insert_tab(buf);
    }

    run(editor, id, Hook::BufChanged);
}

#[action("Insert a tab to each cursor")]
fn backtab(editor: &mut Editor, id: ClientId) {
    run(editor, id, Hook::InsertPre);
    let (win, buf) = editor.win_buf_mut(id);
    if win.cursors().has_selections() {
        win.dedent_cursor_lines(buf);
    } else {
        win.backtab(buf);
    }
    run(editor, id, Hook::BufChanged);
}

#[action("Remove the rest of the line")]
fn remove_line_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.remove_line_after_cursor(buf);
}
