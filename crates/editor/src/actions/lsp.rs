use crate::{editor::Editor, server::ClientId};

use super::jobs::LSP;

#[action("Start LSP")]
fn start(editor: &mut Editor, id: ClientId) {
    let wd = editor.working_dir().to_path_buf();
    let (_win, buf) = editor.win_buf_mut(id);
    let ft = buf.filetype.clone();

    if let Some(ft) = ft {
        let name = format!("LSP-{}", ft.as_str());
        let lsp = LSP::new(id, wd, ft);
        editor.job_broker.request_slot(id, &name, lsp);
    }
}
