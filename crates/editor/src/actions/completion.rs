use std::sync::Arc;

use crate::{
    common::{cursors::non_whitespace_before_cursor, matcher::*},
    editor::{
        windows::{Completion, Focus},
        Editor,
    },
    server::ClientId,
};

use super::{jobs::MatcherJob, text};

#[action("Open completion menu")]
fn complete(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);

    win.completion = Completion::new();
    let cursor = win.primary_cursor();
    if let Some(point) = win.view().point_at_pos(cursor.pos()) {
        win.completion.point = point;
    }

    let opts: Vec<MatchOption> = win
        .syntax_result()
        .highlights
        .iter()
        .filter(|hl| hl.name == "identifier" || hl.name == "string")
        .map(|hl| {
            let compl = String::from(&buf.slice(hl.range.clone()));
            let desc = hl.name.clone();
            MatchOption {
                value: compl,
                description: desc,
            }
        })
        .collect();

    let word = non_whitespace_before_cursor(editor, id).unwrap_or(String::from(""));
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

    let opt = win.completion.selector.selected().map(|opt| {
        let prefix = opt.matches().iter().map(|m| m.end).max().unwrap_or(0);
        opt.value()[prefix..].to_string()
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

#[action("Send word under cursor to matcher")]
fn send_word(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.focus() != Focus::Completion {
        return;
    }

    let (win, buf) = editor.win_buf_mut(id);
    if let Some(fun) = win.completion.on_input.clone() {
        let word = non_whitespace_before_cursor(editor, id).unwrap_or(String::from(""));
        (fun)(editor, id, &word);
    }
}
