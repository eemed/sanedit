use anyhow::{bail, Result};
use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{word_at_pos, ChangesKind, Group, Item};
use sanedit_messages::redraw::{PopupKind, PopupMessage, PopupMessageText};
use sanedit_utils::either::Either;
use std::path::{Path, PathBuf};
use thiserror::Error;

use sanedit_lsp::{LSPRequestError, Notification, Position, PositionRange, RequestKind, TextEdit};

use crate::editor::{
    buffers::{Buffer, BufferConfig, BufferId},
    hooks::Hook,
    lsp::{get_diagnostics, Constraint, LSP},
    windows::{Focus, Prompt, Window},
    Editor,
};

use sanedit_server::ClientId;

use super::{jobs::LSPJob, window::focus, ActionResult};

#[derive(Debug, Error)]
enum LSPActionError {
    #[error("Buffer path is not set")]
    PathNotSet,

    #[error("Buffer not found")]
    BufferNotFound,

    #[error("No language server configured for language {0:?}")]
    LanguageServerNotConfigured(String),

    #[error("Language server is already running for language {0:?}")]
    LanguageServerAlreadyRunning(String),

    #[error("No language server started for language {0:?}")]
    LanguageServerNotStarted(String),

    #[error("Language not set for buffer")]
    LanguageNotSet,
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
    let lang = buf.language.clone().ok_or(LSPActionError::LanguageNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;
    let handle = editor
        .language_servers
        .get(&lang)
        .ok_or_else(|| LSPActionError::LanguageServerNotStarted(lang.as_str().to_string()))?;

    let (win, buf) = editor.win_buf(id);
    let slice = buf.slice(..);
    let request = (f)(win, buf, path, slice, handle);

    if let Some((kind, constraints)) = request {
        let lsp = editor
            .language_servers
            .get_mut(&lang)
            .ok_or_else(|| LSPActionError::LanguageServerNotStarted(lang.as_str().to_string()))?;
        match lsp.request(kind, id, constraints) {
            Err(LSPRequestError::ServerClosed) => {
                editor.language_servers.remove(&lang);
            }
            _ => {}
        }
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
    let lang = buf.language.clone().ok_or(LSPActionError::LanguageNotSet)?;
    let path = buf
        .path()
        .map(Path::to_path_buf)
        .ok_or(LSPActionError::PathNotSet)?;
    let handle = editor
        .language_servers
        .get(&lang)
        .ok_or_else(|| LSPActionError::LanguageServerNotStarted(lang.as_str().to_string()))?;

    let slice = buf.slice(..);
    let request = (f)(buf, path, slice, handle);

    if let Some(notif) = request {
        let lsp = editor
            .language_servers
            .get_mut(&lang)
            .ok_or_else(|| LSPActionError::LanguageServerNotStarted(lang.as_str().to_string()))?;
        match lsp.notify(notif) {
            Err(LSPRequestError::ServerClosed) => {
                editor.language_servers.remove(&lang);
            }
            _ => {}
        }
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

#[action("LSP: Start server")]
fn start_lsp(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;

    if let Err(e) = start_lsp_impl(editor, id, bid) {
        let (win, _buf) = editor.win_buf_mut(id);
        win.error_msg(&format!("{e}"));
        return ActionResult::Failed;
    }

    ActionResult::Ok
}

#[action("LSP: Start server hook")]
fn start_lsp_hook(editor: &mut Editor, id: ClientId) -> ActionResult {
    let bid = getf!(editor.hooks.running_hook().and_then(Hook::buffer_id));
    start_lsp_impl(editor, id, bid).into()
}

fn start_lsp_impl(editor: &mut Editor, id: ClientId, bid: BufferId) -> Result<()> {
    let wd = editor.working_dir().to_path_buf();
    let buf = editor.buffers().get(bid).unwrap();
    let lang = buf.language.clone().ok_or(LSPActionError::LanguageNotSet)?;
    if editor.language_servers.contains_key(&lang) {
        bail!(LSPActionError::LanguageServerAlreadyRunning(
            lang.as_str().to_string()
        ));
    }
    let langconfig = editor
        .languages
        .get(&lang)
        .map(|config| &config.language_server)
        .ok_or_else(|| LSPActionError::LanguageServerNotConfigured(lang.as_str().to_string()))?;

    let lsp = LSPJob::new(id, wd, lang, langconfig);
    editor.job_broker.request(lsp);

    Ok(())
}

#[action("LSP: Stop server")]
fn stop_lsp(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    stop_lsp_impl(editor, id, bid).into()
}

fn stop_lsp_impl(editor: &mut Editor, _id: ClientId, bid: BufferId) -> Result<()> {
    let buf = editor.buffers().get(bid).unwrap();
    let lang = buf.language.clone().ok_or(LSPActionError::LanguageNotSet)?;
    editor.language_servers.remove(&lang);
    Ok(())
}

#[action("LSP: Restart server")]
fn restart_lsp(editor: &mut Editor, id: ClientId) -> ActionResult {
    stop_lsp.execute(editor, id);
    start_lsp.execute(editor, id)
}

#[action("LSP: Hover")]
fn hover(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |win, buf, path, slice, lsp| {
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
    })
    .into()
}

#[action("LSP: Goto definition")]
fn goto_definition(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |win, _buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::GotoDefinition { path, position };
        Some((kind, vec![]))
    })
    .into()
}

#[action("LSP: Workspace symbols to locations")]
fn show_symbols(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |_win, _buf, _path, _slice, _lsp| {
        Some((RequestKind::WorkspaceSymbols, vec![]))
    })
    .into()
}

#[action("LSP: Signature help")]
fn show_signature_help(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |win, buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::SignatureHelp { path, position };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
                Constraint::CursorPosition(offset),
            ],
        ))
    })
    .into()
}

#[action("Synchronize document")]
fn sync_document(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_notify(editor, id, |buf, path, slice, lsp| {
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
                // these need to be calculated in order,
                // so reverse the changes => offsets should not change
                let changes = edit
                    .changes
                    .iter()
                    .rev()
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
    })
    .into()
}

#[action("LSP: Complete")]
fn complete(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |win, buf, path, slice, lsp| {
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
    })
    .into()
}

#[action("LSP: Pull diagnostics")]
fn pull_diagnostics(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |_win, buf, path, _slice, _lsp| {
        let kind = RequestKind::PullDiagnostics { path };
        Some((
            kind,
            vec![
                Constraint::Buffer(buf.id),
                Constraint::BufferVersion(buf.total_changes_made()),
            ],
        ))
    })
    .into()
}

#[action("LSP: Show references")]
fn references(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |win, _buf, path, slice, lsp| {
        let offset = win.cursors.primary().pos();
        let position = Position::new(offset, &slice, &lsp.position_encoding());
        let kind = RequestKind::References { path, position };

        Some((kind, vec![]))
    })
    .into()
}

#[action("LSP: Code action")]
fn code_action(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |win, buf, path, slice, lsp| {
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
    })
    .into()
}

#[action("LSP: Format")]
fn format(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_request(editor, id, move |_win, buf, path, _slice, _lsp| {
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
    })
    .into()
}

#[action("LSP: Rename")]
fn rename(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    let cursor = win.cursors.primary().pos();
    let slice = buf.slice(..);
    let word = getf!(word_at_pos(&slice, cursor));
    let word = String::from(&slice.slice(word));

    win.prompt = Prompt::builder()
        .prompt("Rename to")
        .simple()
        .input(&word)
        .on_confirm(|editor, id, out| {
            let name = getf!(out.text());
            let (win, buf) = editor.win_buf(id);
            let slice = buf.slice(..);
            let offset = win.cursors.primary().pos();
            let total = buf.total_changes_made();
            let bid = buf.id;
            let path = getf!(buf.path().map(Path::to_path_buf));
            let lang = getf!(buf.language.clone());
            let lsp = getf!(editor.language_servers.get(&lang));
            let position = Position::new(offset, &slice, &lsp.position_encoding());
            let request = RequestKind::Rename {
                path,
                position,
                new_name: name.into(),
            };

            let lsp = getf!(editor.language_servers.get_mut(&lang));
            let _ = lsp.request(
                request,
                id,
                vec![Constraint::Buffer(bid), Constraint::BufferVersion(total)],
            );
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Send LSP open document notification")]
pub(crate) fn open_document(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_notify(editor, id, |buf, path, slice, _lsp| {
        let text = String::from(&slice);
        let version = buf.total_changes_made() as i32;
        Some(Notification::DidOpen {
            path: path.clone(),
            text,
            version,
        })
    })
    .into()
}

#[action("Send LSP open document notification")]
pub(crate) fn close_document(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_notify(editor, id, |_buf, path, _slice, _lsp| {
        Some(Notification::DidClose { path: path.clone() })
    })
    .into()
}

#[action("LSP: Show diagnostics on line")]
pub(crate) fn show_diagnostics(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let view = win.view();
    let pos = win.cursors.primary().pos();
    let range = getf!(view.line_at_pos(pos));
    let diagnostics = getf!(get_diagnostics(buf, &editor.language_servers));
    win.clear_popup();

    for diag in diagnostics {
        if range.overlaps(&diag.range()) {
            win.push_popup(
                PopupMessage {
                    severity: Some(*diag.severity()),
                    text: PopupMessageText::Plain(diag.description().to_string()),
                },
                PopupKind::Diagnostic,
            );
        }
    }

    ActionResult::Ok
}

#[action("Will save document notification")]
pub(crate) fn will_save_document(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_notify(editor, id, |_buf, path, _slice, _lsp| {
        Some(Notification::WillSave { path: path.clone() })
    })
    .into()
}

#[action("Did save document notification")]
pub(crate) fn did_save_document(editor: &mut Editor, id: ClientId) -> ActionResult {
    lsp_notify(editor, id, |_buf, path, slice, _lsp| {
        let text = String::from(&slice);
        Some(Notification::DidSave {
            path: path.clone(),
            text: Some(text),
        })
    })
    .into()
}

#[action("LSP: Diagnostics to locations")]
pub(crate) fn diagnostics_to_locations(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let lang = getf!(buf.language.clone());
    let lsp = getf!(editor.language_servers.get(&lang));

    win.locations.clear();

    for (path, diags) in &lsp.diagnostics {
        if diags.is_empty() {
            continue;
        }
        let mut group = Group::new(path);
        for diag in diags.iter() {
            let item = Item::new(
                diag.description(),
                Some(diag.line()),
                Some(diag.range().start),
                vec![],
            );
            group.push(item);
        }
        win.locations.push(group);
    }

    win.locations.show = true;
    focus(editor, id, Focus::Locations);

    ActionResult::Ok
}

#[action("LSP: Jump to next diagnostic")]
pub(crate) fn next_diagnostic(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let lang = getf!(buf.language.clone());
    let lsp = getf!(editor.language_servers.get(&lang));
    let path = getf!(buf.path());
    let diagnostics = getf!(lsp.diagnostics.get(path));

    let cpos = win.cursors().primary().pos();
    for diag in diagnostics.iter() {
        let drange = diag.range();
        if drange.start > cpos {
            win.jump_to_offset(drange.start, buf);
            return ActionResult::Ok;
        }
    }

    ActionResult::Failed
}

#[action("LSP: Jump to previous diagnostic")]
pub(crate) fn prev_diagnostic(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let lang = getf!(buf.language.clone());
    let lsp = getf!(editor.language_servers.get(&lang));
    let path = getf!(buf.path());
    let diagnostics = getf!(lsp.diagnostics.get(path));

    let cpos = win.cursors().primary().pos();
    for diag in diagnostics.iter().rev() {
        let drange = diag.range();
        if drange.end < cpos {
            win.jump_to_offset(drange.start, buf);
            return ActionResult::Ok;
        }
    }

    ActionResult::Failed
}
