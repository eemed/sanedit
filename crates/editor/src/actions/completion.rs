use std::sync::Arc;

use rustc_hash::FxHashSet;
use sanedit_core::{word_before_pos, Range};

use crate::{
    common::matcher::{MatchOption, MatchStrategy},
    editor::{
        hooks::Hook,
        snippets::SNIPPET_DESCRIPTION,
        windows::{Completion, Focus},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{jobs::MatcherJob, lsp, snippets, text};

#[action("Complete")]
fn complete(editor: &mut Editor, id: ClientId) {
    if editor.has_lsp(id) {
        lsp::complete.execute(editor, id)
    } else {
        complete_from_syntax(editor, id);
    }
}

fn complete_from_syntax(editor: &mut Editor, id: ClientId) {
    let (win, buf) = win_buf!(editor, id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let (range, word) =
        word_before_pos(&slice, cursor).unwrap_or((Range::new(cursor, cursor), String::new()));
    let cursor = win.primary_cursor();
    let Some(point) = win.view().point_at_pos(cursor.pos()) else {
        return;
    };
    win.completion = Completion::new(range.start, point);

    // Fetch completions from buffer
    let opts: FxHashSet<MatchOption> = win
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
        .map(|(compl, desc)| MatchOption::with_description(&compl, &desc))
        .collect();

    let opts: Vec<MatchOption> = opts
        .into_iter()
        .chain(
            editor
                .snippets
                .match_options(buf.filetype.as_ref())
                .into_iter(),
        )
        .collect();
    let job = MatcherJob::builder(id)
        .strategy(MatchStrategy::Prefix)
        .search(word)
        .options(Arc::new(opts))
        .handler(Completion::matcher_result_handler)
        .build();

    editor.job_broker.request(job);
}

#[action("Confirm completion")]
fn completion_confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.focus_to(Focus::Window);

    if let Some(opt) = win.completion.selected().cloned() {
        if opt.description() == SNIPPET_DESCRIPTION {
            let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
            snippets::expand_snippet(editor, id, &opt.to_str_lossy(), prefix as u64)
        } else {
            let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
            let opt = opt.to_str_lossy()[prefix..].to_string();
            text::insert(editor, id, &opt);
        }
    }
}

#[action("Abort completion")]
fn completion_abort(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.focus() == Focus::Completion {
        win.focus_to(Focus::Window);
    }
}

#[action("Select next completion")]
fn completion_next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_next();
}

#[action("Select previous completion")]
fn completion_prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_prev();
}

#[action("Send word to matcher")]
fn send_word(editor: &mut Editor, id: ClientId) {
    let hook_bid = editor.hooks.running_hook().and_then(Hook::buffer_id);
    let (win, buf) = editor.win_buf_mut(id);
    if hook_bid != Some(buf.id) || win.focus() != Focus::Completion {
        return;
    }

    let (win, buf) = editor.win_buf_mut(id);
    if let Some(fun) = win.completion.on_input.clone() {
        let cursor = win.cursors.primary().pos();
        let start = win.completion.started_at();
        if start > cursor {
            win.focus_to(Focus::Window);
            return;
        }
        let slice = buf.slice(start..cursor);
        let word = String::from(&slice);
        (fun)(editor, id, &word);
    }
}
