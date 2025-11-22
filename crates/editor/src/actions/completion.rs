use std::sync::Arc;

use rustc_hash::FxHashSet;
use sanedit_core::{word_before_pos, Change, Changes, Range};
use sanedit_utils::either::Either;

use crate::{
    common::Choice,
    editor::{
        hooks::Hook,
        snippets::Snippet,
        windows::{Completion, Focus},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{
    hooks::run,
    jobs::{MatchStrategy, MatcherJob},
    lsp, snippets, text,
    window::focus,
    ActionResult,
};

#[action("Editor: Complete")]
fn complete(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.has_lsp(id) {
        lsp::complete.execute(editor, id)
    } else {
        complete_from_syntax(editor, id)
    }
}

fn complete_from_syntax(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let (range, word) =
        word_before_pos(&slice, cursor).unwrap_or((Range::from(cursor..cursor), String::new()));
    let cursor = win.primary_cursor();
    let point = getf!(win.view().point_at_pos(cursor.pos()));
    win.completion = Completion::new(range.start, cursor.pos(), point);

    // Fetch completions from buffer
    let dyn_compls: FxHashSet<Arc<Choice>> = win
        .view_syntax()
        .spans()
        .iter()
        .filter(|hl| hl.is_completion())
        .map(|hl| {
            let compl = String::from(&buf.slice(hl.range()));
            let desc = hl.completion_category().unwrap_or(hl.name()).to_string();
            (compl, desc)
        })
        .filter(|(compl, _)| compl != &word)
        .map(|(compl, desc)| Choice::from_text_with_description(compl, desc))
        .collect();
    let static_compls = buf
        .language
        .as_ref()
        .and_then(|lang| editor.syntaxes.get(lang).ok())
        .and_then(|syntax| Arc::into_inner(syntax.static_completions()))
        .unwrap_or_default();

    let opts: Vec<Arc<Choice>> = dyn_compls
        .into_iter()
        .chain(static_compls)
        .chain(editor.get_snippets(id))
        .collect();
    let job = MatcherJob::builder(id)
        .strategy(MatchStrategy::Prefix)
        .search(word)
        .options(Arc::new(opts))
        .handler(Completion::matcher_result_handler)
        .build();

    editor.job_broker.request(job);
    ActionResult::Ok
}

#[action("Completion: Confirm")]
fn completion_confirm(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus(editor, id, Focus::Window);

    let (win, buf) = win_buf!(editor, id);
    let opt = getf!(win.completion.selected().cloned());
    let choice = opt.choice();
    match choice {
        Choice::Snippet { snippet, .. } => {
            let pos = win.completion.item_start();
            // Match length
            let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
            return snippets::insert_snippet_impl(
                editor,
                id,
                snippet.clone(),
                Range::from(pos..pos + prefix as u64),
                vec![],
            );
        }
        Choice::LSPCompletion { item, .. } => {
            let lang = getf!(buf.language.clone());
            let enc = getf!(editor
                .language_servers
                .get(&lang)
                .map(|x| x.position_encoding()));
            if item.is_snippet {
                let (text, mut replace) = match item.insert_text() {
                    Either::Left(text) => {
                        let pos = win.completion.point_offset();
                        (text, Range::from(pos..pos))
                    }
                    Either::Right(edit) => {
                        let slice = buf.slice(..);
                        let range = edit.range.to_buffer_range(&slice, &enc);
                        (edit.text.as_str(), range)
                    }
                };

                // Distance between completion triggered and current positions
                let ppos = win.completion.point_offset();
                let cpos = win.primary_cursor().pos();
                if ppos <= cpos {
                    // Added text
                    // | is completion point \ is cursor now
                    //
                    // this.p|ee\
                    // Select peekable()
                    // => Remove "ee" also that the completion
                    // cannot override, because it was triggered at a different point
                    replace.end += cpos - ppos;
                } else {
                    // Deleted text
                    //
                    // this.p\ee|
                    // Select peekable()
                    // => The "ee" is already removed thus shorten the range of completion
                    // So that it does not remove extra data
                    replace.end -= ppos - cpos;
                }
                let snippet = match Snippet::new(text) {
                    Ok(snip) => snip,
                    Err(e) => {
                        log::error!("LSP snippet parse failed: {e}");
                        return ActionResult::Failed;
                    }
                };

                let slice = buf.slice(..);
                let additional_changes: Vec<Change> = item
                    .additional_edits
                    .iter()
                    .filter_map(|edit| {
                        let start = edit.range.start.to_offset(&slice, &enc);
                        let end = if edit.range.start == edit.range.end {
                            start
                        } else {
                            edit.range.end.to_offset(&slice, &enc)
                        };

                        // LSP sometimes may send empty edits for whatever reason
                        if start != end || !edit.text.is_empty() {
                            Some(Change::replace(start..end, edit.text.as_bytes()))
                        } else {
                            None
                        }
                    })
                    .collect();

                return snippets::insert_snippet_impl(
                    editor,
                    id,
                    snippet,
                    replace,
                    additional_changes,
                );
            }

            let (text, mut replace) = match item.insert_text() {
                Either::Left(text) => {
                    let pos = win.completion.point_offset();
                    (text.as_bytes(), Range::from(pos..pos))
                }
                Either::Right(edit) => {
                    let slice = buf.slice(..);
                    let range = edit.range.to_buffer_range(&slice, &enc);
                    (edit.text.as_bytes(), range)
                }
            };

            // See above
            let ppos = win.completion.point_offset();
            let cpos = win.primary_cursor().pos();
            if ppos <= cpos {
                replace.end += cpos - ppos;
            } else {
                replace.end -= ppos - cpos;
            }

            let change = Change::replace(replace, text);
            let changes = Changes::from(vec![change]);
            // let start = changes.iter().next().unwrap().start();

            if win.change(buf, &changes).is_ok() {
                let hook = Hook::BufChanged(buf.id);
                run(editor, id, hook);
            } else {
                return ActionResult::Failed;
            }
        }
        _ => {
            let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
            let opt = choice.text()[prefix..].to_string();
            text::insert(editor, id, &opt)
        }
    }

    ActionResult::Ok
}

#[action("Completion: Abort")]
fn completion_abort(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.focus() == Focus::Completion {
        win.focus = Focus::Window;
        return ActionResult::Ok;
    }

    ActionResult::Skipped
}

#[action("Completion: Select next")]
fn completion_next(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_next();

    ActionResult::Ok
}

#[action("Completion: Select previous")]
fn completion_prev(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_prev();

    ActionResult::Ok
}

#[action("Send word to matcher")]
fn send_word(editor: &mut Editor, id: ClientId) -> ActionResult {
    let hook_bid = editor.hooks.running_hook().and_then(Hook::buffer_id);
    let (win, buf) = editor.win_buf_mut(id);
    if hook_bid != Some(buf.id) || win.focus() != Focus::Completion {
        return ActionResult::Skipped;
    }

    let (win, buf) = editor.win_buf_mut(id);
    if let Some(fun) = win.completion.on_input.clone() {
        let cursor = win.cursors.primary().pos();
        let start = win.completion.item_start();
        if start > cursor {
            focus(editor, id, Focus::Window);
            return ActionResult::Ok;
        }
        let slice = buf.slice(start..cursor);
        let word = String::from(&slice);
        (fun)(editor, id, &word);
    }

    ActionResult::Ok
}
