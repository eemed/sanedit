use crate::{
    editor::{
        windows::{Completion, Focus},
        Editor,
    },
    server::ClientId,
};

#[action("Open completion menu")]
fn complete(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.completion = Completion::new();
    win.focus = Focus::Completion;

    provide(editor, id, vec!["hello".into(), "world".into()]);
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

pub(crate) fn provide(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.provide_options(completions);
}
