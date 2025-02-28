use std::sync::Arc;

use rustc_hash::FxHashSet;
use sanedit_core::{word_before_pos, Range};

use crate::{
    common::matcher::{Choice, MatchStrategy},
    editor::{
        hooks::Hook,
        windows::{Completion, Focus},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{jobs::MatcherJob, lsp, snippets, text};

#[action("Editor: Complete")]
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
    win.completion = Completion::new(range.start, point, Some(&win.keymap_layer));

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

    let opts: Vec<Arc<Choice>> = opts
        .into_iter()
        .chain(editor.snippets.match_options(buf.filetype.as_ref()))
        .collect();
    let job = MatcherJob::builder(id)
        .strategy(MatchStrategy::Prefix)
        .search(word)
        .options(Arc::new(opts))
        .handler(Completion::matcher_result_handler)
        .build();

    editor.job_broker.request(job);
}

#[action("Completion: Confirm")]
fn completion_confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.pop_focus();

    if let Some(opt) = win.completion.selected().cloned() {
        let choice = opt.choice();
        match choice {
            Choice::Snippet { snippet, .. } => {
                let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
                snippets::insert_snippet_impl(editor, id, snippet.clone(), prefix as u64)
            }
            _ => {
                let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
                let opt = choice.text()[prefix..].to_string();
                text::insert(editor, id, &opt);
            }
        }
    }
}

#[action("Completion: Abort")]
fn completion_abort(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.focus() == Focus::Completion {
        win.pop_focus();

        if let Some(ref km) = win.completion.previous_keymap {
            win.keymap_layer = km.into();
        }
    }
}

#[action("Completion: Select next")]
fn completion_next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_next();
}

#[action("Completion: Select previous")]
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
            win.pop_focus();
            return;
        }
        let slice = buf.slice(start..cursor);
        let word = String::from(&slice);
        (fun)(editor, id, &word);
    }
}
