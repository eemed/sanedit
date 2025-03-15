use std::sync::Arc;

use rustc_hash::FxHashSet;
use sanedit_core::{word_before_pos, Change, Changes, Cursor, Range};
use sanedit_utils::either::Either;

use crate::{
    common::matcher::{Choice, MatchStrategy},
    editor::{
        hooks::Hook,
        snippets::Snippet,
        windows::{Completion, Cursors, Focus},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{hooks::run, jobs::MatcherJob, lsp, snippets, text, window::pop_focus, ActionResult};

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
        word_before_pos(&slice, cursor).unwrap_or((Range::new(cursor, cursor), String::new()));
    let cursor = win.primary_cursor();
    let point = getf!(win.view().point_at_pos(cursor.pos()));
    win.completion = Completion::new(range.start, point);

    // Fetch completions from buffer
    let opts: FxHashSet<Arc<Choice>> = win
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

    let opts: Vec<Arc<Choice>> = opts.into_iter().chain(editor.get_snippets(id)).collect();
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
    pop_focus(editor, id);

    let (win, buf) = win_buf!(editor, id);

    if let Some(opt) = win.completion.selected().cloned() {
        let choice = opt.choice();
        match choice {
            Choice::Snippet { snippet, .. } => {
                let pos = win.completion.started_at();
                let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
                snippets::insert_snippet_impl(
                    editor,
                    id,
                    snippet.clone(),
                    Range::new(pos, pos + prefix as u64),
                )
            }
            Choice::LSPCompletion { item } => {
                if item.is_snippet {
                    let (text, replace) = match item.insert_text() {
                        Either::Left(text) => {
                            let pos = win.completion.started_at();
                            let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
                            (text, Range::new(pos, pos + prefix as u64))
                        }
                        Either::Right(edit) => {
                            let ft = getf!(buf.filetype.clone());
                            let enc = getf!(editor
                                .language_servers
                                .get(&ft)
                                .map(|x| x.position_encoding()));
                            let slice = buf.slice(..);
                            let range = edit.range.to_buffer_range(&slice, &enc);
                            (edit.text.as_str(), range)
                        }
                    };
                    log::info!("Snippet: {:?}", item.insert_text());
                    let snippet = match Snippet::new(text) {
                        Ok(snip) => snip,
                        Err(e) => {
                            log::error!("LSP snippet parse failed: {e}");
                            return ActionResult::Failed;
                        }
                    };

                    snippets::insert_snippet_impl(editor, id, snippet, replace);
                    return ActionResult::Ok;
                }

                match item.insert_text() {
                    Either::Left(text) => {
                        let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
                        let opt = text[prefix..].to_string();
                        text::insert(editor, id, &opt)
                    }
                    Either::Right(edit) => {
                        let ft = getf!(buf.filetype.clone());
                        let enc = getf!(editor
                            .language_servers
                            .get(&ft)
                            .map(|x| x.position_encoding()));
                        let bid = buf.id.clone();
                        let slice = buf.slice(..);
                        let change = {
                            let range = edit.range.to_buffer_range(&slice, &enc);
                            Change::replace(range, edit.text.as_bytes())
                        };
                        let changes = Changes::from(vec![change]);
                        let start = changes.iter().next().unwrap().start();

                        match buf.apply_changes(&changes) {
                            Ok(result) => {
                                if let Some(id) = result.created_snapshot {
                                    if let Some(aux) = buf.snapshot_aux_mut(id) {
                                        aux.cursors = Cursors::new(Cursor::new(start));
                                        aux.view_offset = start;
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("LSP text edit failed: {e}");
                                return ActionResult::Failed;
                            }
                        }

                        run(editor, id, Hook::BufChanged(bid));
                    }
                }
            }
            _ => {
                let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
                let opt = choice.text()[prefix..].to_string();
                text::insert(editor, id, &opt)
            }
        }
    }

    ActionResult::Ok
}

#[action("Completion: Abort")]
fn completion_abort(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.focus() == Focus::Completion {
        pop_focus(editor, id);
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
        let start = win.completion.started_at();
        if start > cursor {
            pop_focus(editor, id);
            return ActionResult::Ok;
        }
        let slice = buf.slice(start..cursor);
        let word = String::from(&slice);
        (fun)(editor, id, &word);
    }

    ActionResult::Ok
}
