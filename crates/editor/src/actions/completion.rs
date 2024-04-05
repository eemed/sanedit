use crate::{
    editor::{
        windows::{Completion, Focus, SelectorOption},
        Editor,
    },
    server::ClientId,
};

#[action("Open completion menu")]
fn complete(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.completion = Completion::new();
    let cursor = win.primary_cursor();
    if let Some(point) = win.view().point_at_pos(cursor.pos()) {
        win.completion.point = point;
    }

    win.focus = Focus::Completion;

    let opts: Vec<SelectorOption> = ["hello", "world", "longer line"]
        .into_iter()
        .enumerate()
        .map(|(i, s)| SelectorOption::new(s.to_string(), vec![], i as u32))
        .collect();
    win.completion.provide_options(opts.into());
}

#[action("Confirm completion")]
fn confirm(_editor: &mut Editor, _id: ClientId) {}

#[action("Abort completion")]
fn abort(_editor: &mut Editor, _id: ClientId) {}

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
