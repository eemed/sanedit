use crate::{
    editor::{
        windows::{Completion, Focus},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn complete(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);

    win.completion = Completion::new();
    win.focus = Focus::Completion;

    provide(editor, id, vec!["hello".into(), "world".into()]);
}

pub(crate) fn confirm(editor: &mut Editor, id: ClientId) {}
pub(crate) fn abort(editor: &mut Editor, id: ClientId) {}

pub(crate) fn next(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.completion.select_next();
}

pub(crate) fn prev(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.completion.select_prev();
}

pub(crate) fn provide(editor: &mut Editor, id: ClientId, completions: Vec<String>) {
    let (win, buf) = editor.win_buf_mut(id);
    win.completion.provide_options(completions);
}
