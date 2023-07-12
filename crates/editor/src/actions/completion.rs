use crate::{
    editor::{
        windows::{Completion, Focus},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn complete(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    win.completion = Completion::new();
    win.focus = Focus::Completion;

    provide(editor, id, vec!["hello".into(), "world".into()]);
}

pub(crate) fn confirm(_editor: &mut Editor, _id: ClientId) {}
pub(crate) fn abort(_editor: &mut Editor, _id: ClientId) {}

pub(crate) fn next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_next();
}

pub(crate) fn prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.select_prev();
}

pub(crate) fn provide(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.completion.provide_options(completions);
}
