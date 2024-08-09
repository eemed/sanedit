use crate::{editor::Editor, server::ClientId};

use super::jobs::LSP;

#[action("Start LSP for current filetype")]
fn start(editor: &mut Editor, id: ClientId) {
    let wd = editor.working_dir().to_path_buf();
    let lsp = LSP::new(id, wd);
    editor.job_broker.request_slot(id, "lsp", lsp);
}
