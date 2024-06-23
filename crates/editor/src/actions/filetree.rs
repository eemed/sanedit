use crate::{
    editor::{windows::Focus, Editor},
    server::ClientId,
};

#[action("Show filetree")]
fn show_filetree(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Filetree;
}
