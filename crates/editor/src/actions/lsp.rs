use anyhow::{anyhow, bail, Result};
use sanedit_buffer::PieceTreeSlice;
use sanedit_core::ChangesKind;
use sanedit_messages::redraw::PopupMessage;
use sanedit_utils::either::Either;
use std::path::{Path, PathBuf};
use thiserror::Error;

use sanedit_lsp::{Notification, Position, PositionRange, RequestKind, TextEdit};

use crate::{
    common::cursors::word_at_cursor,
    editor::{
        buffers::{Buffer, BufferConfig, BufferId},
        hooks::Hook,
        windows::{Focus, Prompt, Window},
        Editor,
    },
};

use sanedit_server::ClientId;

use super::jobs::{Constraint, LSPHandle, LSP};

#[derive(Debug, Error)]
enum LSPActionError {
    #[error("Buffer path is not set")]
    PathNotSet,

    #[error("No language server configured for filetype {0:?}")]
    LanguageServerNotConfigured(String),

    #[error("Language server is already running for filetype {0:?}")]
    LanguageServerAlreadyRunning(String),

    #[error("No language server started for filetype {0:?}")]
    LanguageServerNotStarted(String),

    #[error("Filetype not set for buffer")]
    FiletypeNotSet,
}

/// Helper function to get out all of the mostly used stuff from editor state
pub(crate) fn lsp_request(
    editor: &mut Editor,
    id: ClientId,
    f: fn(
        &Window,
        &Buffer,
        PathBuf,
        PieceTreeSlice,
        &LSPHandle,
    ) -> Option<(RequestKind, Vec<Constraint>)>,
) -> Result<()> {
    let (_win, buf) = editor.win_buf_mut(id);
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;
    let handle = editor
        .language_servers
        .get(&ft)
        .ok_or_else(|| LSPActionError::LanguageServerNotStarted(ft.as_str().to_string()))?;

    let (win, buf) = editor.win_buf(id);
    let slice = buf.slice(..);
    let request = (f)(win, buf, path, slice, handle);

    if let Some((kind, constraints)) = request {
        let lsp = editor
            .language_servers
            .get_mut(&ft)
            .ok_or_else(|| LSPActionError::LanguageServerNotStarted(ft.as_str().to_string()))?;
        lsp.request(kind, id, constraints)
    } else {
        Ok(())
    }
}

#[action("Start language server")]
fn start_lsp(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;

    if let Err(e) = start_lsp_impl(editor, id, bid) {
        let (win, _buf) = editor.win_buf_mut(id);
        win.error_msg(&format!("{e}"));
    }
}

#[action("Start language server from hook")]
fn start_lsp_hook(editor: &mut Editor, id: ClientId) {
    if let Some(bid) = editor.hooks.running_hook().and_then(Hook::buffer_id) {
        let _ = start_lsp_impl(editor, id, bid);
    }
}

fn start_lsp_impl(editor: &mut Editor, id: ClientId, bid: BufferId) -> Result<()> {
    let wd = editor.working_dir().to_path_buf();
    let buf = editor.buffers().get(bid).unwrap();
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    if editor.language_servers.contains_key(&ft) {
        bail!(LSPActionError::LanguageServerAlreadyRunning(
            ft.as_str().to_string()
        ));
    }
    let lang = editor
        .config
        .editor
        .language_server
        .get(ft.as_str())
        .ok_or_else(|| LSPActionError::LanguageServerNotConfigured(ft.as_str().to_string()))?;

    let lsp = LSP::new(id, wd, ft, lang);
    editor.job_broker.request(lsp);

    Ok(())
}

#[action("Stop language server")]
fn stop_lsp(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let _ = stop_lsp_impl(editor, id, bid);
}

fn stop_lsp_impl(editor: &mut Editor, _id: ClientId, bid: BufferId) -> Result<()> {
    let buf = editor.buffers().get(bid).unwrap();
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    editor.language_servers.remove(&ft);
    Ok(())
}

#[action("Restart language server")]
fn restart_lsp(editor: &mut Editor, id: ClientId) {
    stop_lsp.execute(editor, id);
    start_lsp.execute(editor, id);
}

#[action("Hover information")]
fn hover(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::Hover { path, position };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
                Constraint::CursorPosition(offset),
            ],
        ))
    });
}

#[action("Goto definition")]
fn goto_definition(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, _buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::GotoDefinition { path, position };
        Some((kind, vec![]))
    });
}

#[action("Synchronize document")]
fn sync_document(editor: &mut Editor, id: ClientId) {
    fn sync(editor: &mut Editor, bid: BufferId) -> Result<()> {
        let buf = editor.buffers().get(bid).unwrap();
        let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
        let path = buf
            .path()
            .map(Path::to_path_buf)
            .ok_or(LSPActionError::PathNotSet)?;

        let version = buf.total_changes_made() as i32;
        let Some(edit) = buf.last_edit() else {
            // Nothing to sync
            return Ok(());
        };
        let lsp = editor
            .language_servers
            .get(&ft)
            .ok_or_else(|| LSPActionError::LanguageServerNotStarted(ft.as_str().to_string()))?;
        let enc = lsp.position_encoding();
        let slice = edit.buf.slice(..);

        use ChangesKind::*;
        let changes = match edit.changes.kind() {
            Undo | Redo => Either::Right(String::from(&buf.slice(..))),
            _ => {
                let changes = edit
                    .changes
                    .iter()
                    .map(|change| {
                        let start = Position::new(change.start(), &slice, &enc);
                        let end = if change.range().is_empty() {
                            start.clone()
                        } else {
                            Position::new(change.end(), &slice, &enc)
                        };

                        TextEdit {
                            range: PositionRange { start, end },
                            text: String::from_utf8(change.text().into())
                                .expect("Change was not UTF8"),
                        }
                    })
                    .collect();
                Either::Left(changes)
            }
        };

        let lsp = editor
            .language_servers
            .get_mut(&ft)
            .ok_or_else(|| LSPActionError::LanguageServerNotStarted(ft.as_str().to_string()))?;
        lsp.notify(Notification::DidChange {
            path,
            changes,
            version,
        })
    }

    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);

    let _ = sync(editor, bid);
}

#[action("Complete")]
fn complete(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::Complete { path, position };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
                Constraint::CursorPosition(offset),
            ],
        ))
    });
}

#[action("Pull diagnostics")]
fn pull_diagnostics(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |_win, buf, path, _slice, _lsp| {
        let kind = RequestKind::PullDiagnostics { path };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
            ],
        ))
    });
}

#[action("Show references")]
fn references(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, _buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::References { path, position };

        Some((kind, vec![]))
    });
}

#[action("Code action")]
fn code_action(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::CodeAction { path, position };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
            ],
        ))
    });
}

#[action("Format")]
fn format(editor: &mut Editor, id: ClientId) {
    let _ = lsp_request(editor, id, move |_win, buf, path, _slice, _lsp| {
        let BufferConfig {
            indent_kind,
            indent_amount,
            ..
        } = buf.config;
        let kind = RequestKind::Format {
            path,
            indent_kind,
            indent_amount: indent_amount.into(),
        };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
            ],
        ))
    });
}

#[action("Rename")]
fn rename(editor: &mut Editor, id: ClientId) {
    let Some(word) = word_at_cursor(editor, id) else {
        return;
    };
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Rename to")
        .simple()
        .input(&word)
        .on_confirm(|editor, id, input| {
            let (win, buf) = editor.win_buf(id);
            let slice = buf.slice(..);
            let offset = win.cursors.primary().pos();
            let total = buf.total_changes_made();
            let bid = buf.id;
            let Some(path) = buf.path().map(Path::to_path_buf) else {
                return;
            };
            let Some(ft) = buf.filetype.clone() else {
                return;
            };
            let Some(lsp) = editor.language_servers.get(&ft) else {
                return;
            };
            let position = Position::new(offset, &slice, &lsp.position_encoding());
            let request = RequestKind::Rename {
                path,
                position,
                new_name: input.into(),
            };

            let Some(lsp) = editor.language_servers.get_mut(&ft) else {
                return;
            };
            let _ = lsp.request(
                request,
                id,
                vec![Constraint::Buffer(bid), Constraint::BufferVersion(total)],
            );
        })
        .build();
    win.focus = Focus::Prompt;
}

#[action("Send LSP open document notification")]
pub(crate) fn open_doc(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let _ = open_document(editor, bid);
}

#[action("Send LSP open document notification")]
pub(crate) fn close_doc(editor: &mut Editor, id: ClientId) {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let _ = close_document(editor, bid);
}

#[action("Show diagnostics on line")]
pub(crate) fn show_diagnostics(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let view = win.view();
    let pos = win.cursors.primary().pos();

    if let Some(range) = view.line_at_pos(pos) {
        for diag in buf.diagnostics.iter() {
            if range.overlaps(&diag.range()) {
                win.push_popup(PopupMessage {
                    severity: Some(*diag.severity()),
                    text: diag.description().to_string(),
                });
            }
        }
    }
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

    let lsp =
        editor
            .language_servers
            .get_mut(&ft)
            .ok_or(LSPActionError::LanguageServerNotStarted(
                ft.as_str().to_string(),
            ))?;
    lsp.notify(Notification::DidOpen {
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

    let lsp =
        editor
            .language_servers
            .get_mut(&ft)
            .ok_or(LSPActionError::LanguageServerNotStarted(
                ft.as_str().to_string(),
            ))?;
    lsp.notify(Notification::DidClose { path: path.clone() })
}

#[action("Sync document on save")]
pub(crate) fn sync_on_save(editor: &mut Editor, id: ClientId) {
    close_doc.execute(editor, id);
    open_doc.execute(editor, id);
}
