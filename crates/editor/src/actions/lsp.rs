use anyhow::{anyhow, bail, Result};
use sanedit_buffer::PieceTreeSlice;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

use sanedit_lsp::{lsp_types, Request};

use crate::{
    editor::{
        buffers::{Buffer, BufferId, ChangesKind, Filetype},
        windows::Window,
        Editor,
    },
    server::ClientId,
};

use super::jobs::{LSPHandle, LSP};

#[derive(Debug, Error)]
enum LSPActionError {
    #[error("Buffer path is not set")]
    PathNotSet,

    #[error("No language server configured for filetype {0:?}")]
    LanguageServerNotConfigured(Filetype),

    #[error("Language server is already running for filetype {0:?}")]
    LanguageServerAlreadyRunning(Filetype),

    #[error("No language server started for filetype {0:?}")]
    LanguageServerNotStarted(Filetype),

    #[error("File type not set for buffer")]
    FiletypeNotSet,
}

/// Helper function to get out all of the mostly used stuff from editor state
pub(crate) fn lsp_request(
    editor: &mut Editor,
    id: ClientId,
    f: fn(&Window, &Buffer, PathBuf, PieceTreeSlice, &LSPHandle) -> Option<Request>,
) -> Result<()> {
    let (win, buf) = editor.win_buf_mut(id);
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;
    let handle = editor
        .language_servers
        .get(&ft)
        .ok_or(LSPActionError::LanguageServerNotStarted(ft.clone()))?;

    let (win, buf) = editor.win_buf(id);
    let slice = buf.slice(..);
    let request = (f)(win, buf, path, slice, &handle);

    if let Some(request) = request {
        let lsp = editor
            .language_servers
            .get_mut(&ft)
            .ok_or(LSPActionError::LanguageServerNotStarted(ft.clone()))?;
        lsp.send(request)
    } else {
        Ok(())
    }
}

#[action("Start language server")]
fn start_lsp(editor: &mut Editor, id: ClientId) {
    fn start_lsp(editor: &mut Editor, id: ClientId) -> Result<()> {
        let wd = editor.working_dir().to_path_buf();
        let (_win, buf) = editor.win_buf_mut(id);
        let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
        if editor.language_servers.get(&ft).is_some() {
            bail!(LSPActionError::LanguageServerAlreadyRunning(ft.clone()));
        }
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
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = offset_to_position(&slice, offset, &lsp.position_encoding());
        Request::Hover { path, position }.into()
    });
}

#[action("Goto definition")]
fn goto_definition(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = offset_to_position(&slice, offset, &lsp.position_encoding());
        Request::GotoDefinition { path, position }.into()
    });
}

#[action("Synchronize document")]
fn sync_document(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let version = buf.total_changes_made() as i32;
        let edit = buf.last_edit()?;
        let enc = lsp.position_encoding();

        use ChangesKind::*;
        let changes = match edit.changes.kind() {
            Undo | Redo => {
                vec![sanedit_lsp::Change {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: offset_to_position(&slice, slice.len(), &enc),
                    text: String::from(&slice),
                }]
            }
            _ => edit
                .changes
                .iter()
                .map(|change| sanedit_lsp::Change {
                    start: offset_to_position(&slice, change.start(), &enc),
                    end: offset_to_position(&slice, change.end(), &enc),
                    text: String::from_utf8(change.text().into()).expect("Change was not UTF8"),
                })
                .collect(),
        };

        Request::DidChange {
            path,
            changes,
            version,
        }
        .into()
    });
}

#[action("Complete")]
fn complete(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = offset_to_position(&slice, offset, &lsp.position_encoding());
        Request::Complete { path, position }.into()
    });
}

#[action("Show references")]
fn references(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = offset_to_position(&slice, offset, &lsp.position_encoding());
        Request::References { path, position }.into()
    });
}

#[action("Code action")]
fn code_action(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = offset_to_position(&slice, offset, &lsp.position_encoding());
        Request::CodeAction { path, position }.into()
    });
}

#[action("Send LSP open document notification")]
pub(crate) fn open_doc(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf(id);
    open_document(editor, win.buffer_id());
}

#[action("Send LSP open document notification")]
pub(crate) fn close_doc(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf(id);
    close_document(editor, win.buffer_id());
}

pub(crate) fn open_document(editor: &mut Editor, bid: BufferId) -> Result<()> {
    let buf = editor
        .buffers()
        .get(bid)
        .ok_or(anyhow!("Buffer not found"))?;
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;

    let text = String::from(&buf.slice(..));
    let version = buf.total_changes_made() as i32;

    let lsp = editor
        .language_servers
        .get_mut(&ft)
        .ok_or(LSPActionError::LanguageServerNotStarted(ft.clone()))?;
    lsp.send(Request::DidOpen {
        path: path.clone(),
        text,
        version,
    })
}

pub(crate) fn close_document(editor: &mut Editor, bid: BufferId) -> Result<()> {
    let buf = editor
        .buffers()
        .get(bid)
        .ok_or(anyhow!("Buffer not found"))?;
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;

    let lsp = editor
        .language_servers
        .get_mut(&ft)
        .ok_or(LSPActionError::LanguageServerNotStarted(ft.clone()))?;
    lsp.send(Request::DidClose { path: path.clone() })
}

pub(crate) fn offset_to_position(
    slice: &PieceTreeSlice,
    mut offset: u64,
    kind: &lsp_types::PositionEncodingKind,
) -> lsp_types::Position {
    let (row, line) = slice.line_at(offset);
    offset -= line.start();

    let mut chars = line.chars();
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if start > offset {
            break;
        }
        let len = if *kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if *kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if *kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    lsp_types::Position {
        line: row as u32,
        character: col,
    }
}

pub(crate) fn position_to_offset(
    slice: &PieceTreeSlice,
    position: lsp_types::Position,
    kind: &lsp_types::PositionEncodingKind,
) -> u64 {
    let lsp_types::Position { line, character } = position;
    let pos = slice.pos_at_line(line as u64);
    let mut chars = slice.chars_at(pos);
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if col >= character {
            return start;
        }
        let len = if *kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if *kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if *kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    unreachable!("Position not found")
}
