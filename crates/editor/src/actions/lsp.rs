use std::path::Path;

use sanedit_lsp::Operation;

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

#[action("LSP Hover")]
fn hover(editor: &mut Editor, id: ClientId) {
    let wd = editor.working_dir().to_path_buf();
    let (win, buf) = editor.win_buf_mut(id);
    let ft = buf.filetype.clone();
    let Some(path) = buf.path().map(Path::to_path_buf) else {
        return;
    };
    let buf = buf.read_only_copy();
    let offset = win.cursors.primary().pos();

    let Some(ft) = ft else {
        return;
    };
    let Some(lsp) = editor.language_servers.get_mut(&ft) else {
        return;
    };

    lsp.send(Operation::Hover { path, buf, offset });
}
