use crate::{editor::Editor, server::ClientId};

#[action("Reload the current window")]
fn reload(editor: &mut Editor, id: ClientId) {
    editor.reload(id);
}
