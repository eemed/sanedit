use crate::{editor::Editor, server::ClientId};

#[action("Quit Sanedit")]
fn quit(editor: &mut Editor, _id: ClientId) {
    editor.quit();
}
