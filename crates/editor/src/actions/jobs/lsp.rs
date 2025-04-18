use std::{
    any::Any,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    actions::{hooks::run, locations, lsp::lsp_notify_for},
    common::matcher::{Choice, MatchStrategy},
    editor::{
        buffers::BufferId,
        config::LSPConfig,
        hooks::Hook,
        job_broker::KeepInTouch,
        lsp::{Constraint, LSP},
        windows::{Completion, Cursors, Prompt, Zone},
        Editor,
    },
};
use sanedit_buffer::{PieceTree, PieceTreeSlice};
use sanedit_core::{
    word_before_pos, Change, Changes, Cursor, Diagnostic, Filetype, Group, Item, Range,
};
use sanedit_lsp::{
    CodeAction, CompletionItem, FileEdit, LSPClientParams, Notification, Position,
    PositionEncoding, PositionRange, RequestKind, RequestResult, Response, TextDiagnostic,
    WorkspaceEdit,
};

use sanedit_messages::redraw::PopupMessage;
use sanedit_server::{ClientId, Job, JobContext, JobResult};

use super::MatcherJob;

#[derive(Debug)]
enum Message {
    Started(LSP),
    Response(Response),
}

#[derive(Clone)]
pub(crate) struct LSPJob {
    client_id: ClientId,
    filetype: Filetype,
    working_dir: PathBuf,
    opts: LSPConfig,
}

impl LSPJob {
    pub fn new(id: ClientId, working_dir: PathBuf, ft: Filetype, opts: &LSPConfig) -> LSPJob {
        LSPJob {
            client_id: id,
            filetype: ft,
            working_dir,
            opts: opts.clone(),
        }
    }
}

impl Job for LSPJob {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        // Clones here
        let wd = self.working_dir.clone();
        let ft = self.filetype.clone();
        let opts = self.opts.clone();

        let fut = async move {
            log::info!("Run rust-analyzer");
            let LSPConfig { command, args } = opts;
            let filetype: String = ft.as_str().into();
            let params = LSPClientParams {
                run_command: command.clone(),
                run_args: args,
                root: wd.clone(),
                filetype,
            };

            let (sender, mut reader) = params.spawn().await?;

            let handle = LSP::new(&command, sender);
            ctx.send(Message::Started(handle));

            while let Some(response) = reader.recv().await {
                ctx.send(Message::Response(response));
            }

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for LSPJob {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(output) = msg.downcast::<Message>() {
            match *output {
                Message::Started(sender) => {
                    if editor.language_servers.contains_key(&self.filetype) {
                        log::error!(
                            "Language server for {} is already running.",
                            self.filetype.as_str()
                        );
                        // Shutsdown automatically because sender is dropped
                        // here
                        return;
                    }
                    // Set sender
                    editor
                        .language_servers
                        .insert(self.filetype.clone(), sender);

                    // Send all buffers of this filetype
                    let ids: Vec<BufferId> = editor.buffers().iter().map(|(id, _buf)| id).collect();
                    for id in ids {
                        let _ = lsp_notify_for(editor, id, |buf, path, slice, _lsp| {
                            let text = String::from(&slice);
                            let version = buf.total_changes_made() as i32;
                            Some(Notification::DidOpen {
                                path: path.clone(),
                                text,
                                version,
                            })
                        });
                    }
                }
                Message::Response(response) => self.handle_response(editor, response),
            }
        }
    }
}

impl LSPJob {
    fn handle_response(&self, editor: &mut Editor, response: Response) {
        match response {
            Response::Request { id, result } => {
                let lsp = get!(editor.language_servers.get_mut(&self.filetype));
                let (cid, constraints) = get!(lsp.reponse_of(id));

                // Verify constraints
                let (win, buf) = editor.win_buf(cid);
                for constraint in constraints {
                    let ok = match constraint {
                        Constraint::Buffer(bid) => win.buffer_id() == bid,
                        Constraint::BufferVersion(v) => buf.total_changes_made() == v,
                        Constraint::CursorPosition(pos) => win.cursors.primary().pos() == pos,
                    };

                    if !ok {
                        log::debug!(
                            "LSP response {result:?} filtered because constraint {constraint:?}"
                        );
                        return;
                    }
                }

                self.handle_result(editor, cid, result);
            }
            Response::Notification(notif) => match notif {
                sanedit_lsp::NotificationResult::Diagnostics {
                    path,
                    version,
                    diagnostics,
                } => self.handle_diagnostics(editor, path, version, diagnostics),
            },
        }
    }

    fn handle_result(&self, editor: &mut Editor, id: ClientId, result: RequestResult) {
        match result {
            RequestResult::Hover { text, .. } => {
                let (win, _buf) = editor.win_buf_mut(id);
                win.push_popup(PopupMessage {
                    severity: None,
                    text,
                });
            }
            RequestResult::GotoDefinition { path, position } => {
                if editor.open_file(id, path).is_ok() {
                    let enc = get!(editor.lsp_for(id).map(|x| x.position_encoding()));
                    let (win, buf) = editor.win_buf_mut(self.client_id);
                    let slice = buf.slice(..);
                    let offset = position.to_offset(&slice, &enc);
                    win.goto_offset(offset, buf);
                    win.view_to_cursor_zone(buf, Zone::Middle);
                }
            }
            RequestResult::Complete {
                path,
                position,
                results,
            } => self.complete(editor, id, path, position, results),
            RequestResult::References { references } => {
                self.show_references(editor, id, references)
            }
            RequestResult::CodeAction { actions } => self.code_action(editor, id, actions),
            RequestResult::ResolvedAction { action } => {
                if let Some(edit) = action.workspace_edit() {
                    Self::edit_workspace(editor, id, edit)
                }
            }
            RequestResult::Rename { workspace_edit } => {
                Self::edit_workspace(editor, id, workspace_edit)
            }
            RequestResult::Format { edit } => Self::edit_document(editor, id, edit),
            RequestResult::Error { msg } => {
                log::error!("LSP '{}' failed to process: {msg}", self.opts.command);
            }
            RequestResult::Diagnostics { path, diagnostics } => {
                self.handle_diagnostics(editor, path, None, diagnostics)
            }
        }
    }

    fn complete(
        &self,
        editor: &mut Editor,
        id: ClientId,
        _path: PathBuf,
        position: Position,
        opts: Vec<CompletionItem>,
    ) {
        let enc = get!(editor.lsp_for(id).map(|x| x.position_encoding()));
        let (win, buf) = win_buf!(editor, id);
        let slice = buf.slice(..);
        let start = position.to_offset(&slice, &enc);
        let cursor = win.primary_cursor();
        let point = get!(win.view().point_at_pos(cursor.pos()));
        let (range, word) =
            word_before_pos(&slice, start).unwrap_or((Range::new(start, start), String::default()));

        win.completion = Completion::new(range.start, cursor.pos(), point);

        let opts: Vec<Arc<Choice>> = opts
            .into_iter()
            .map(Choice::from_completion_item)
            .chain(editor.get_snippets(id))
            .collect();

        let job = MatcherJob::builder(id)
            .strategy(MatchStrategy::Prefix)
            .options(Arc::new(opts))
            .search(word)
            .handler(Completion::matcher_result_handler)
            .build();

        editor.job_broker.request(job);
    }

    fn handle_diagnostics(
        &self,
        editor: &mut Editor,
        path: PathBuf,
        version: Option<i32>,
        diags: Vec<TextDiagnostic>,
    ) {
        let Some(bid) = editor.buffers().find(&path) else {
            return;
        };
        let buf = editor.buffers().get(bid).unwrap();

        // Ensure not changed
        if let Some(version) = version {
            if buf.total_changes_made() != version as u32 {
                return;
            }
        }

        let Some(enc) = editor
            .language_servers
            .get(&self.filetype)
            .map(|x| x.position_encoding())
        else {
            return;
        };

        let buf = editor.buffers().get(bid).unwrap();
        let slice = buf.slice(..);
        let diagnostics = diags
            .into_iter()
            .map(|d| {
                let start;
                let end;
                if d.range.start == d.range.end {
                    start = d.range.start.to_offset(&slice, &enc);
                    end = start + 1;
                } else {
                    start = d.range.start.to_offset(&slice, &enc);
                    end = d.range.end.to_offset(&slice, &enc);
                }
                Diagnostic::new(d.severity, (start..end).into(), d.line, &d.description)
            })
            .collect();

        let lsp = editor.language_servers.get_mut(&self.filetype).unwrap();
        lsp.diagnostics.insert(path, diagnostics);
    }

    fn code_action(&self, editor: &mut Editor, id: ClientId, actions: Vec<CodeAction>) {
        let options: Vec<Arc<Choice>> = actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                Choice::from_numbered_text((i + 1) as u32, action.name().to_string())
            })
            .collect();
        let (win, _buf) = editor.win_buf_mut(id);

        if options.is_empty() {
            win.warn_msg("No code actions available");
            return;
        }

        let job = MatcherJob::builder(id)
            .options(Arc::new(options))
            .handler(Prompt::matcher_result_handler)
            .build();

        let ft = self.filetype.clone();
        win.prompt = Prompt::builder()
            .prompt("Select code action")
            .on_confirm(move |editor, id, out| {
                let n = get!(out.number()) as usize;
                let action = &actions[n - 1];

                if action.is_resolved() {
                    let edit = action.workspace_edit().unwrap();
                    Self::edit_workspace(editor, id, edit)
                } else {
                    let request = RequestKind::CodeActionResolve {
                        action: action.clone(),
                    };

                    let lsp = get!(editor.language_servers.get_mut(&ft));
                    let _ = lsp.request(request, id, vec![]);
                }
            })
            .build();

        editor.job_broker.request(job);
    }

    fn edit_workspace(editor: &mut Editor, id: ClientId, edit: WorkspaceEdit) {
        for edit in edit.file_edits {
            Self::edit_document(editor, id, edit);
        }
    }

    fn edit_document(editor: &mut Editor, id: ClientId, edit: FileEdit) {
        let path = &edit.path;
        let bid = match editor.buffers().find(&path) {
            Some(bid) => bid,
            None => match editor.create_buffer(id, &path) {
                Ok(bid) => bid,
                Err(e) => {
                    log::error!("Failed to create buffer for {path:?} {e}");
                    return;
                }
            },
        };
        let enc = get!(editor.lsp_for(id).map(|x| x.position_encoding()));

        let buf = editor.buffers_mut().get_mut(bid).unwrap();
        let slice = buf.slice(..);
        let changes: Vec<Change> = edit
            .edits
            .into_iter()
            .map(|edit| {
                let start = edit.range.start.to_offset(&slice, &enc);
                let end = if edit.range.end != edit.range.start {
                    edit.range.end.to_offset(&slice, &enc)
                } else {
                    start
                };
                Change::replace((start..end).into(), edit.text.as_bytes())
            })
            .collect();
        let changes = Changes::from(changes);
        let start = changes.iter().next().unwrap().start();

        match buf.apply_changes(&changes) {
            Ok(result) => {
                if let Some(id) = result.created_snapshot {
                    if let Some(aux) = buf.snapshot_aux_mut(id) {
                        aux.cursors = Cursors::new(Cursor::new(start));
                        aux.view_offset = start;
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to apply changes to buffer: {path:?}: {e}");
                return;
            }
        }

        // We may have edited the buffer in the window
        // Hook only runs this for all other windows than self
        let (win, buf) = editor.win_buf_mut(id);
        if buf.id == bid {
            win.on_buffer_changed(buf);
        }

        run(editor, id, Hook::BufChanged(bid));
    }

    fn show_references(
        &self,
        editor: &mut Editor,
        id: ClientId,
        references: BTreeMap<PathBuf, Vec<PositionRange>>,
    ) {
        let Some(enc) = editor.lsp_for(id).map(|handle| handle.position_encoding()) else {
            return;
        };

        // TODO should this be auto shown?
        locations::show_locations.execute(editor, id);

        let (win, buf) = editor.win_buf_mut(id);

        for (path, references) in references {
            if buf.path() == Some(&path) {
                let slice = buf.slice(..);
                let group = read_references(&slice, &path, &references, &enc);
                win.locations.push(group);
            } else if let Ok(pt) = PieceTree::from_path(&path) {
                let slice = pt.slice(..);
                let group = read_references(&slice, &path, &references, &enc);
                win.locations.push(group);
            }
        }
    }
}

fn read_references(
    slice: &PieceTreeSlice,
    path: &Path,
    references: &[PositionRange],
    enc: &PositionEncoding,
) -> Group {
    let mut group = Group::new(path);

    for re in references {
        let start = re.start.to_offset(slice, enc);
        let end = re.end.to_offset(slice, enc);
        let (row, line) = slice.line_at(start);
        let lstart = line.start();

        let hlstart = (start - lstart) as usize;
        let hlend = (end - lstart) as usize;
        let text = String::from(&line);
        let text = text.trim_end();
        let item = Item::new(
            text,
            Some(row),
            Some(lstart),
            vec![Range::new(hlstart, hlend)],
        );

        group.push(item);
    }

    group
}
