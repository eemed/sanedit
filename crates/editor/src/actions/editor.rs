use crate::{editor::Editor, server::ClientId};

pub(crate) fn quit(editor: &mut Editor, _id: ClientId) {
    editor.quit();
}
