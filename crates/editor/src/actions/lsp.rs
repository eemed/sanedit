use anyhow::{anyhow, Result};
use std::path::Path;
use thiserror::Error;

use sanedit_lsp::{Position, Request};

use crate::{
    editor::{
        buffers::{ChangesKind, Filetype},
        Editor,
    },
    server::ClientId,
};

use super::jobs::LSP;

#[derive(Debug, Error)]
enum LSPActionError {
    #[error("Buffer path is not set")]
    PathNotSet,

    #[error("No language server configured for filetype {0:?}")]
    LanguageServerNotConfigured(Filetype),

    #[error("File type not set for buffer")]
    FiletypeNotSet,
}

#[action("Start language server")]
fn start(editor: &mut Editor, id: ClientId) {
    fn start_lsp(editor: &mut Editor, id: ClientId) -> Result<()> {
        let wd = editor.working_dir().to_path_buf();
        let (_win, buf) = editor.win_buf_mut(id);
        let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
        let lang = editor
            .options
            .language_server
            .get(ft.as_str())
            .ok_or(LSPActionError::LanguageServerNotConfigured(ft.clone()))?;

        let name = format!("LSP-{}", ft.as_str());
        let lsp = LSP::new(id, wd, ft, lang);
        editor.job_broker.request_slot(id, &name, lsp);

        Ok(())
    }

    if let Err(e) = start_lsp(editor, id) {
        let (win, buf) = editor.win_buf_mut(id);
        win.error_msg(&format!("{e}"));
    }
}

#[action("Hover information")]
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

    lsp.send(Request::Hover {
        path,
        buf,
        position: Position::Offset(offset),
    });
}

#[action("Goto definition")]
fn goto_definition(editor: &mut Editor, id: ClientId) {
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

    lsp.send(Request::GotoDefinition {
        path,
        buf,
        position: Position::Offset(offset),
    });
}

#[action("Synchronize document")]
fn sync_document(editor: &mut Editor, id: ClientId) {
    let wd = editor.working_dir().to_path_buf();
    let (win, buf) = editor.win_buf_mut(id);
    let ft = buf.filetype.clone();
    let Some(path) = buf.path().map(Path::to_path_buf) else {
        return;
    };
    let copy = buf.read_only_copy();
    let offset = win.cursors.primary().pos();

    let Some(ft) = ft else {
        return;
    };

    let Some(changes) = buf.last_changes() else {
        return;
    };

    use ChangesKind::*;
    let changes = match changes.kind() {
        Undo | Redo => {
            vec![sanedit_lsp::Change {
                start: Position::Offset(0),
                end: Position::Offset(buf.len()),
                text: String::from(&buf.slice(..)),
            }]
        }
        _ => changes
            .iter()
            .map(|change| sanedit_lsp::Change {
                start: Position::Offset(change.start()),
                end: Position::Offset(change.end()),
                text: String::from_utf8(change.text().into()).expect("Change was not UTF8"),
            })
            .collect(),
    };

    let Some(lsp) = editor.language_servers.get_mut(&ft) else {
        return;
    };
    lsp.send(Request::DidChange {
        path,
        buf: copy,
        changes,
    });
}

#[action("Complete")]
fn complete(editor: &mut Editor, id: ClientId) {
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

    lsp.send(Request::Complete {
        path,
        buf,
        position: Position::Offset(offset),
    });
}
