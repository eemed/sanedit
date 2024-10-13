use anyhow::{bail, Result};
use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{word_at_pos, ChangesKind, Group, Item};
use sanedit_messages::redraw::PopupMessage;
use sanedit_utils::either::Either;
use std::path::{Path, PathBuf};
use thiserror::Error;

use sanedit_lsp::{Notification, Position, PositionRange, RequestKind, TextEdit};

use crate::{
    editor::{
        buffers::{Buffer, BufferConfig, BufferId},
        hooks::Hook,
        lsp::{get_diagnostics, Constraint, LSP},
        windows::{Focus, Prompt, Window},
        Editor,
    },
    get, win_buf,
};

use sanedit_server::ClientId;

use super::jobs::LSPJob;

#[derive(Debug, Error)]
enum LSPActionError {
    #[error("Buffer path is not set")]
    PathNotSet,

    #[error("Buffer not found")]
    BufferNotFound,

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
        &LSP,
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
        lsp.request(kind, id, constraints)?;
    }

    Ok(())
}

pub(crate) fn lsp_notify_for(
    editor: &mut Editor,
    bid: BufferId,
    f: fn(&Buffer, PathBuf, PieceTreeSlice, &LSP) -> Option<Notification>,
) -> Result<()> {
    let buf = editor
        .buffers()
        .get(bid)
        .ok_or(LSPActionError::BufferNotFound)?;
    let ft = buf.filetype.clone().ok_or(LSPActionError::FiletypeNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;
    let handle = editor
        .language_servers
        .get(&ft)
        .ok_or_else(|| LSPActionError::LanguageServerNotStarted(ft.as_str().to_string()))?;

    let slice = buf.slice(..);
    let request = (f)(buf, path, slice, handle);

    if let Some(notif) = request {
        let lsp = editor
            .language_servers
            .get_mut(&ft)
            .ok_or_else(|| LSPActionError::LanguageServerNotStarted(ft.as_str().to_string()))?;
        lsp.notify(notif)?;
    }

    Ok(())
}

pub(crate) fn lsp_notify(
    editor: &mut Editor,
    id: ClientId,
    f: fn(&Buffer, PathBuf, PieceTreeSlice, &LSP) -> Option<Notification>,
) -> Result<()> {
    let (_win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    lsp_notify_for(editor, bid, f)
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

    let lsp = LSPJob::new(id, wd, ft, lang);
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
    let _ = lsp_notify(editor, id, |buf, path, slice, lsp| {
        let version = buf.total_changes_made() as i32;
        let Some(edit) = buf.last_edit() else {
            // Nothing to sync
            return None;
        };
        let enc = lsp.position_encoding();
        let eslice = edit.buf.slice(..);

        use ChangesKind::*;
        let changes = match edit.changes.kind() {
            Undo | Redo => Either::Right(String::from(&slice)),
            _ => {
                let changes = edit
                    .changes
                    .iter()
                    .map(|change| {
                        let start = Position::new(change.start(), &eslice, &enc);
                        let end = if change.range().is_empty() {
                            start.clone()
                        } else {
                            Position::new(change.end(), &eslice, &enc)
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

        Some(Notification::DidChange {
            path,
            changes,
            version,
        })
    });
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
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let word = get!(word_at_pos(&slice, cursor));
    let word = String::from(&slice.slice(word));

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
            let path = get!(buf.path().map(Path::to_path_buf));
            let ft = get!(buf.filetype.clone());
            let lsp = get!(editor.language_servers.get(&ft));
            let position = Position::new(offset, &slice, &lsp.position_encoding());
            let request = RequestKind::Rename {
                path,
                position,
                new_name: input.into(),
            };

            let lsp = get!(editor.language_servers.get_mut(&ft));
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
pub(crate) fn open_document(editor: &mut Editor, id: ClientId) {
    let _ = lsp_notify(editor, id, |buf, path, slice, _lsp| {
        let text = String::from(&slice);
        let version = buf.total_changes_made() as i32;
        Some(Notification::DidOpen {
            path: path.clone(),
            text,
            version,
        })
    });
}

#[action("Send LSP open document notification")]
pub(crate) fn close_document(editor: &mut Editor, id: ClientId) {
    let _ = lsp_notify(editor, id, |_buf, path, _slice, _lsp| {
        Some(Notification::DidClose { path: path.clone() })
    });
}

#[action("Show diagnostics on line")]
pub(crate) fn show_diagnostics(editor: &mut Editor, id: ClientId) {
    let (win, buf) = win_buf!(editor, id);
    let view = win.view();
    let pos = win.cursors.primary().pos();
    let range = get!(view.line_at_pos(pos));
    let diagnostics = get!(get_diagnostics(buf, &editor.language_servers));

    for diag in diagnostics {
        if range.overlaps(&diag.range()) {
            win.push_popup(PopupMessage {
                severity: Some(*diag.severity()),
                text: diag.description().to_string(),
            });
        }
    }
}

#[action("Will save document notification")]
pub(crate) fn will_save_document(editor: &mut Editor, id: ClientId) {
    let _ = lsp_notify(editor, id, |_buf, path, _slice, _lsp| {
        Some(Notification::WillSave { path: path.clone() })
    });
}

#[action("Did save document notification")]
pub(crate) fn did_save_document(editor: &mut Editor, id: ClientId) {
    let _ = lsp_notify(editor, id, |_buf, path, slice, _lsp| {
        let text = String::from(&slice);
        Some(Notification::DidSave {
            path: path.clone(),
            text: Some(text),
        })
    });
}

#[action("Diagnostics to locations")]
pub(crate) fn diagnostics_to_locations(editor: &mut Editor, id: ClientId) {
    let (win, buf) = win_buf!(editor, id);
    let ft = get!(buf.filetype.clone());
    let lsp = get!(editor.language_servers.get(&ft));

    win.locations.clear();

    for (path, diags) in &lsp.diagnostics {
        let mut group = Group::new(path);
        for diag in diags.iter() {
            let item = Item::new(diag.description(), None, Some(diag.range().start), vec![]);
            log::info!("push: {item:?}");
            group.push(item);
        }
        win.locations.push(group);
    }

    win.locations.show = true;
    win.focus = Focus::Locations;
}
