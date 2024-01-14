use crate::{common::text_objects::find_range, editor::Editor, server::ClientId};

#[action("Select in curly brackets")]
fn select_curly(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);
    let pos = win.cursors.primary().pos();
    let range = find_range(&slice, pos, "{", "}", false);

    match range {
        Some(range) => {
            if !range.is_empty() {
                win.cursors.primary_mut().select(range);
            }
        }
        None => log::info!("No range found"),
    }
}
