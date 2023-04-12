use crate::{
    common,
    editor::{
        buffers::Buffer,
        windows::{Layer, Window},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.remove_grapheme_after_cursor(buf);
}

pub(crate) fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    win.remove_grapheme_before_cursor(buf);
}

pub(crate) fn undo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn redo(editor: &mut Editor, id: ClientId) {}

pub(crate) fn insert(editor: &mut Editor, id: ClientId, text: &str) {
    let (win, buf) = editor.get_win_buf_mut(id);

    // Find possible layer that wants the key
    use Layer::*;
    for layer in win.layers_mut() {
        if layer.handle_insert(text) {
            match layer {
                Prompt(p) => {
                    if let Some((on_input, input)) = p.get_on_input() {
                        (on_input)(editor, id, &input);
                    }
                }
                Search(s) => {
                    if let Some((on_input, input)) = s.prompt().get_on_input() {
                        (on_input)(editor, id, &input);
                    }
                }
            }

            return;
        }
    }

    win.insert_at_cursor(buf, text);
}
