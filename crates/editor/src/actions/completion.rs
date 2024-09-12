use std::sync::Arc;

use rustc_hash::FxHashSet;

use crate::{
    common::{
        cursors::word_before_cursor,
        matcher::{MatchOption, MatchStrategy},
    },
    editor::{
        hooks::Hook,
        windows::{Completion, Focus},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::{jobs::MatcherJob, text};

#[action("Open completion menu")]
fn complete(editor: &mut Editor, id: ClientId) {
    let result = word_before_cursor(editor, id);
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary().pos();
    let start = result.as_ref().map(|result| result.0).unwrap_or(cursor);
    let word = result.map(|result| result.2).unwrap_or(String::new());
    let cursor = win.primary_cursor();
    let Some(point) = win.view().point_at_pos(cursor.pos()) else {
        return;
    };
    win.completion = Completion::new(start, point);

    let opts: FxHashSet<MatchOption> = win
        .syntax_result()
        .highlights
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

    let opts: Vec<MatchOption> = opts.into_iter().collect();
    let job = MatcherJob::builder(id)
        .strategy(MatchStrategy::Prefix)
        .search(word)
        .options(Arc::new(opts))
        .handler(Completion::matcher_result_handler)
        .build();

    editor.job_broker.request(job);
}

#[action("Confirm completion")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;

    let opt = win.completion.selected().map(|opt| {
        let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
        opt.to_str_lossy()[prefix..].to_string()
    });

    if let Some(opt) = opt {
        text::insert(editor, id, &opt);
    }
}

#[action("Abort completion")]
fn abort(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Select next completion")]
fn next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_next();
}

#[action("Select previous completion")]
fn prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_prev();
}

#[action("Send word to matcher")]
fn send_word(editor: &mut Editor, id: ClientId) {
    let hook_bid = editor.hooks.running_hook().map(Hook::buffer_id).flatten();
    let (win, buf) = editor.win_buf_mut(id);
    if hook_bid != Some(buf.id) || win.focus() != Focus::Completion {
        return;
    }

    let (win, buf) = editor.win_buf_mut(id);
    if let Some(fun) = win.completion.on_input.clone() {
        let cursor = win.cursors.primary().pos();
        let start = win.completion.started_at();
        let slice = buf.slice(start..cursor);
        let word = String::from(&slice);
        (fun)(editor, id, &word);
    }
}
