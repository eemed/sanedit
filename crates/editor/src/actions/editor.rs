use crate::{editor::Editor, server::ClientId};

#[action("Quit Sanedit")]
fn quit(editor: &mut Editor, id: ClientId) {
    // If is the first client
    if id.0 == 0 {
        editor.quit();
    } else {
        editor.quit_client(id);
    }
}
