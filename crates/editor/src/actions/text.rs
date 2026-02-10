use std::{mem, sync::Arc};

use sanedit_buffer::utf8::EndOfLine;
use sanedit_core::{at_start_of_line, is_indent_at_pos, IndentKind, Language};

use crate::{
    actions::movement::start_of_buffer,
    common::is_yes,
    editor::{
        buffers::{Buffer, BufferError, BufferId},
        hooks::Hook,
        language::Languages,
        windows::{Focus, NextKeyFunction, Prompt, ViewSyntax, Window},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{
    completion,
    cursors::{remove_cursor_selections, swap_selection_dir},
    hooks::run,
    jobs::MatcherJob,
    movement::{end_of_line, prev_line},
    text_objects::{select_line, select_line_content},
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
        _ => {}
    }
}

#[action("Buffer: Save all buffers")]
fn save_all(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf(id);
    let original = buf.id;
    let bids: Vec<BufferId> = editor.buffers().iter().map(|(key, _)| key).collect();

    // Save all buffers using this window
    for bid in bids {
        let buf = editor.buffers.get(bid).unwrap();
        if buf.is_modified() {
            editor.open_buffer(id, bid);
            save.execute(editor, id);
        }
    }

    // Return to original buffer
    editor.open_buffer(id, original);

    ActionResult::Ok
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
}

#[action("Buffer: Save as")]
fn save_as(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Save as")
        .simple()
        .on_confirm(|editor, id, out| {
            let mut path = getf!(out.path());
            let (_win, buf) = editor.win_buf_mut(id);
            let bid = buf.id;
            if path.is_relative() {
                path = editor.working_dir().join(path);
            }

            if path.exists() {
                let (win, _buf) = editor.win_buf_mut(id);
                win.error_msg(&format!("File already exists: {path:?}"));
                return ActionResult::Failed;
            }

            // Leave the current buffer
            run(editor, id, Hook::BufLeave(bid));
            run(editor, id, Hook::BufDeletedPre(bid));

            // Set path
            let (_win, buf) = editor.win_buf_mut(id);
            buf.set_path(&path);

            // Rejoin buffer
            run(editor, id, Hook::BufCreated(bid));
            run(editor, id, Hook::BufEnter(bid));

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
) -> Option<(&'a str, &'a str)> {
    let Some(lang) = &buf.language else {
        if show_error {
            win.warn_msg("No language set");
        }
        return None;
    };
    let Some(langconfig) = languages.get(lang) else {
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

    Some((comment, &langconfig.comment_end))
}

#[action("Buffer: Comment lines")]
fn comment_lines(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    match get_comment(&editor.languages, win, buf, true) {
        Some((comment, end)) => {
            if win.comment_cursor_lines(buf, comment, end).is_ok() {
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
        Some((comment, end)) => {
            if win.uncomment_cursor_lines(buf, comment, end).is_ok() {
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
        Some((comment, end)) => {
            if win.toggle_comment_cursor_lines(buf, comment, end).is_ok() {
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
    let (comment, com_end) = get_comment(&editor.languages, win, buf, false).unwrap_or(("", ""));

    if win.join_lines(buf, comment, com_end).is_ok() {
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

#[action("Buffer: Change line")]
fn change_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    select_line_content.execute(editor, id);
    remove_cursor_selections.execute(editor, id);
    mode_insert(editor, id);
    ActionResult::Ok
}

#[action("Buffer: Change to line end")]
fn change_to_eol(editor: &mut Editor, id: ClientId) -> ActionResult {
    remove_to_eol.execute(editor, id);
    mode_insert(editor, id);
    ActionResult::Ok
}

#[action("Buffer: Check if file has been modified")]
fn check_file_modification(editor: &mut Editor, id: ClientId) -> ActionResult {
    let prompt = editor.config.editor.auto_reload_changed_or_removed_file;
    let (win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;

    let path = getf!(buf.path());
    let in_fs = path.metadata().ok().and_then(|mdata| mdata.modified().ok());
    let local = getf!(buf.last_saved_modified.as_ref());

    // File is deleted
    if in_fs.is_none() {
        if buf.is_file_backed() {
            win.error_msg("Filebacked buffer was removed from disk.");
            let _ = editor.remove_buffer(id, bid);
        } else {
            if !prompt {
                let _ = editor.remove_buffer(id, bid);
                return ActionResult::Ok;
            }

            win.prompt = Prompt::builder()
                .prompt("File has been removed from disk. Keep buffer anyway? (y/N)")
                .simple()
                .on_confirm(|editor, id, out| {
                    let input = getf!(out.text());
                    let yes = is_yes(input);
                    let (_win, buf) = editor.win_buf_mut(id);
                    let bid = buf.id;
                    if yes {
                        buf.set_unsaved();
                    } else {
                        let _ = editor.remove_buffer(id, bid);
                    }
                    ActionResult::Ok
                })
                .build();
            focus(editor, id, Focus::Prompt);
        }
        return ActionResult::Ok;
    }

    let in_fs = in_fs.unwrap();
    if local != &in_fs {
        // Prevent asking all the time
        buf.last_saved_modified = Some(in_fs);

        if !prompt {
            return reload_file_from_disk.execute(editor, id);
        }

        win.prompt = Prompt::builder()
            .prompt("File has been changed on disk. Reload file from disk? (Y/n)")
            .simple()
            .on_confirm(|editor, id, out| {
                let input = getf!(out.text());
                let yes = input.is_empty() || is_yes(input);
                if !yes {
                    return ActionResult::Failed;
                }
                reload_file_from_disk.execute(editor, id)
            })
            .build();
        focus(editor, id, Focus::Prompt);
    }

    ActionResult::Ok
}

#[action("Buffer: Reload file from disk")]
fn reload_file_from_disk(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let hook = Hook::BufDeletedPre(buf.id);
    run(editor, id, hook);

    let (_win, buf) = editor.win_buf_mut(id);
    let ok = buf.reload_from_disk();
    if !ok {
        return ActionResult::Ok;
    }

    let hook = Hook::BufCreated(bid);
    run(editor, id, hook);

    let hook = Hook::BufEnter(bid);
    run(editor, id, hook);

    // Reload all clients that use this buffer
    let clients = editor.windows().find_clients_with_buf(bid);
    for client in clients {
        let (win, buf) = editor.win_buf_mut(client);
        win.full_reload(buf);
    }

    ActionResult::Ok
}

#[action("Buffer: Selections to uppercase")]
fn uppercase(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.uppercase_selections(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}

#[action("Buffer: Selections to lowercase")]
fn lowercase(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.lowercase_selections(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}

#[action("Buffer: Rotate selections")]
fn rotate_selections(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.rotate_selections(buf, false).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}

#[action("Buffer: Rotate selections backwards")]
fn rotate_selections_backwards(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.rotate_selections(buf, true).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}

#[action("Buffer: Set language")]
fn set_language(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Language")
        .simple()
        .on_confirm(|editor, id, input| {
            let text = getf!(input.text());
            let lang = Language::new(text);
            editor.load_language(&lang, false);

            let (win, buf) = editor.win_buf_mut(id);
            buf.language = Some(lang);
            *win.view_syntax() = ViewSyntax::default();

            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);

    ActionResult::Ok
}

#[action("Buffer: Set indent")]
fn set_indentation(editor: &mut Editor, id: ClientId) -> ActionResult {
    const TAB: &str = "Tab";
    const SPACE: &str = "Space";
    let (win, _buf) = editor.win_buf_mut(id);

    let options: Arc<Vec<String>> = Arc::new(vec![TAB.into(), SPACE.into()]);
    let job = MatcherJob::builder(id)
        .options(options)
        .handler(Prompt::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Indent kind")
        .loads_options()
        .simple()
        .on_confirm(|editor, id, input| match getf!(input.text()) {
            SPACE => indentation_amount(editor, id, IndentKind::Space),
            TAB => indentation_amount(editor, id, IndentKind::Tab),
            _ => ActionResult::Failed,
        })
        .build();
    focus(editor, id, Focus::Prompt);
    editor.job_broker.request(job);

    ActionResult::Ok
}

fn indentation_amount(editor: &mut Editor, id: ClientId, kind: IndentKind) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.prompt = Prompt::builder()
        .prompt("Indent amount")
        .input(&format!("{}", buf.config.indent_amount))
        .simple()
        .on_confirm(move |editor, id, input| {
            let n = getf!(input.number());
            let (_win, buf) = editor.win_buf_mut(id);
            buf.config.indent_kind = kind;
            buf.config.indent_amount = n as u8;
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);

    ActionResult::Ok
}

#[action("Buffer: Reindent")]
fn redindent(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.reindent(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}

#[action("Buffer: Set end of line")]
fn set_eol(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    let eols: Vec<&'static str> = EndOfLine::all()
        .iter()
        .map(|eol| match eol {
            EndOfLine::Lf => "LF",
            EndOfLine::Vt => "VT",
            EndOfLine::Ff => "FF",
            EndOfLine::Cr => "CR",
            EndOfLine::Crlf => "CRLF",
            EndOfLine::Nel => "NEL",
            EndOfLine::Ls => "LS",
            EndOfLine::Ps => "PS",
        })
        .collect();
    let options: Arc<Vec<&'static str>> = Arc::new(eols);
    let job = MatcherJob::builder(id)
        .options(options.clone())
        .handler(Prompt::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Line ending")
        .loads_options()
        .simple()
        .on_confirm(move |editor, id, input| {
            let (_win, buf) = editor.win_buf_mut(id);
            let input = getf!(input.text());
            for (i, eol) in options.iter().enumerate() {
                if *eol == input {
                    buf.config.eol = EndOfLine::all()[i];
                    return ActionResult::Ok;
                }
            }

            ActionResult::Failed
        })
        .build();
    focus(editor, id, Focus::Prompt);
    editor.job_broker.request(job);

    ActionResult::Ok
}

#[action("Buffer: Fix end of lines")]
fn fix_eols(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.set_eols(buf).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}
