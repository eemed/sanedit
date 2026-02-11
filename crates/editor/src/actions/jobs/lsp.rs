use std::{
    any::Any,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    actions::{
        filetree::{handle_file_delete, handle_file_rename},
        hooks::run,
        locations,
        lsp::lsp_notify_for,
        ActionResult,
    },
    common::{markdown, Choice},
    editor::{
        buffers::BufferId,
        config::LSPConfig,
        hooks::Hook,
        job_broker::KeepInTouch,
        lsp::{Constraint, Lsp},
        windows::{Completion, Cursors, Prompt},
        Editor,
    },
};
use sanedit_buffer::{PieceTree, PieceTreeSlice};
use sanedit_core::{
    prev_non_word, Change, Changes, Cursor, Diagnostic, GraphemeCategory, Group, Item, Language,
    Range,
};
use sanedit_lsp::{
    CodeAction, CompletionItem, FileEdit, FileOperation, LSPClientParams, LSPClientSender,
    Notification, Position, PositionEncoding, PositionRange, RequestKind, RequestResult, Response,
    Signatures, Symbol, Text, TextDiagnostic, TextKind, WorkspaceEdit,
};

use sanedit_messages::redraw::{PopupKind, PopupMessage, PopupMessageText};
use sanedit_server::{ClientId, Job, JobContext, JobResult};

use super::{MatchStrategy, MatcherJob};

#[derive(Debug)]
enum Message {
    Started(LSPClientSender),
    Response(Box<Response>),
}

#[derive(Clone)]
pub(crate) struct LSPJob {
    client_id: ClientId,
    language: Language,
    working_dir: PathBuf,
    opts: LSPConfig,
}

impl LSPJob {
    pub fn new(id: ClientId, working_dir: PathBuf, lang: Language, opts: &LSPConfig) -> LSPJob {
        LSPJob {
            client_id: id,
            language: lang,
            working_dir,
            opts: opts.clone(),
        }
    }
}

impl Job for LSPJob {
    fn run(&self, ctx: JobContext) -> JobResult {
        // Clones here
        let wd = self.working_dir.clone();
        let lang = self.language.clone();
        let opts = self.opts.clone();

        let fut = async move {
            log::info!("Run LSP {:?}", opts);
            let LSPConfig { command, args } = opts;
            let lang: String = lang.as_str().into();
            let params = LSPClientParams {
                run_command: command.clone(),
                run_args: args,
                root: wd.clone(),
                language: lang,
            };

            let (sender, mut reader) = params.spawn().await?;

            ctx.send(Message::Started(sender));

            while let Some(response) = reader.recv().await {
                ctx.send(Message::Response(Box::new(response)));
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
                    let Some(lsp) = editor.language_servers.get_mut(&self.language) else {
                        return;
                    };
                    lsp.start(sender);

                    // Send all buffers of this language
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
                Message::Response(response) => self.handle_response(editor, *response),
            }
        }
    }

    fn on_success(&self, editor: &mut Editor) {
        editor.language_servers.remove(&self.language);
    }

    fn on_failure(&self, editor: &mut Editor, _reason: &str) {
        editor.language_servers.remove(&self.language);
    }

    fn on_stop(&self, editor: &mut Editor) {
        editor.language_servers.remove(&self.language);
    }
}

impl LSPJob {
    fn handle_response(&self, editor: &mut Editor, response: Response) {
        match response {
            Response::Request { id, result } => {
                let lsp = get!(editor.language_servers.get_mut(&self.language));
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
                        log::trace!(
                            "LSP response {result:?} filtered because constraint {constraint:?}"
                        );
                        return;
                    }
                }

                self.handle_result(editor, cid, *result);
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

    fn hover(&self, editor: &mut Editor, id: ClientId, texts: Vec<Text>) {
        let markdown = Language::new("markdown");
        editor.load_language(&markdown, false);
        let syntax = editor.syntaxes.get(&markdown);
        let (win, _buf) = win_buf!(editor, id);
        let theme = {
            let theme_name = &win.config.theme;
            editor.themes.get(theme_name).expect("Invalid theme")
        };
        win.clear_popup();

        for text in texts {
            match text.kind {
                TextKind::Plain => {
                    win.push_popup(
                        PopupMessage {
                            severity: None,
                            text: PopupMessageText::Plain(text.text),
                        },
                        PopupKind::Hover,
                    );
                }
                TextKind::Markdown => match &syntax {
                    Ok(syn) => {
                        let text = markdown::render_markdown_to_popup(text.text, syn, theme);
                        win.push_popup(
                            PopupMessage {
                                severity: None,
                                text,
                            },
                            PopupKind::Hover,
                        );
                    }
                    _ => {
                        win.push_popup(
                            PopupMessage {
                                severity: None,
                                text: PopupMessageText::Plain(text.text),
                            },
                            PopupKind::Hover,
                        );
                    }
                },
            }
        }
    }

    fn goto_definition(
        &self,
        editor: &mut Editor,
        id: ClientId,
        path: PathBuf,
        position: Position,
    ) {
        log::info!("GOTO DEF");
        let (win, buf) = editor.win_buf_mut(id);
        win.push_new_cursor_jump(buf);

        let is_current = buf.path().map(|p| p == path).unwrap_or(false);
        if !is_current && editor.open_file(id, path).is_err() {
            return;
        }

        let enc = get!(editor.lsp_for(id).and_then(Lsp::position_encoding));
        let (win, buf) = editor.win_buf_mut(id);
        let slice = buf.slice(..);
        let offset = position.to_offset(&slice, &enc);
        win.goto_offset(offset, buf);
    }

    fn handle_result(&self, editor: &mut Editor, id: ClientId, result: RequestResult) {
        let mut on_message_post = true;

        match result {
            RequestResult::Hover { texts, .. } => self.hover(editor, id, texts),
            RequestResult::GotoDefinition { path, position } => {
                self.goto_definition(editor, id, path, position)
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
                self.handle_diagnostics(editor, path, None, diagnostics);
                on_message_post = false;
            }
            RequestResult::WorkspaceSymbols { symbols } => {
                self.handle_workspace_symbols(editor, id, symbols);
            }
            RequestResult::SignatureHelp { signatures } => {
                self.handle_signature_help(editor, id, signatures);
            }
        }

        // TODO better solution, this currently highlights syntax / searches
        // and is disabled for if lsp is spamming the response type for example for diagnostics
        if on_message_post {
            run(editor, id, Hook::OnMessagePost);
        }
    }

    fn handle_signature_help(&self, editor: &mut Editor, id: ClientId, signatures: Signatures) {
        let (win, _buf) = editor.win_buf_mut(id);
        for signature in signatures.signatures {
            let text = signature.name;
            let msg = PopupMessage {
                severity: None,
                text: PopupMessageText::Plain(text),
            };
            win.push_popup(msg, PopupKind::SignatureHelp);
        }
    }

    fn handle_workspace_symbols(
        &self,
        editor: &mut Editor,
        id: ClientId,
        symbols: BTreeMap<PathBuf, Vec<Symbol>>,
    ) {
        let (win, _buf) = editor.win_buf_mut(id);
        win.locations.clear();
        locations::show_locations.execute(editor, id);

        let enc = get!(editor.lsp_for(id).and_then(Lsp::position_encoding));
        let (win, _buf) = win_buf!(editor, id);

        for (path, symbols) in symbols {
            let mut group = Group::new(&path);
            for sym in symbols {
                let name = format!("{} {}", sym.kind.as_ref(), sym.name);
                let offset = if let Some(buf) = editor.buffers.find(&path) {
                    let buf = editor.buffers.get(buf).unwrap();
                    let slice = buf.slice(..);
                    sym.position.to_offset(&slice, &enc)
                } else if let Ok(pt) = PieceTree::from_path(&path) {
                    let slice = pt.slice(..);
                    sym.position.to_offset(&slice, &enc)
                } else {
                    continue;
                };
                let item = Item::new(&name, None, Some(offset), vec![]);
                group.push(item);
            }
            win.locations.push(group);
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
        let enc = get!(editor.lsp_for(id).and_then(Lsp::position_encoding));
        let (win, buf) = win_buf!(editor, id);
        let slice = buf.slice(..);
        let start = position.to_offset(&slice, &enc);
        let cursor = win.primary_cursor();
        let point = get!(win.view().point_at_pos(cursor.pos()));
        let (start, cat) = prev_non_word(&slice, start);
        let word = {
            let slice = slice.slice(start..cursor.pos());
            String::from(&slice)
        };
        let add_own_snippets = matches!(
            cat,
            Some(GraphemeCategory::Whitespace) | Some(GraphemeCategory::Eol)
        );

        win.completion = Completion::new(start, cursor.pos(), point);

        let mut opts: Vec<Arc<Choice>> =
            opts.into_iter().map(Choice::from_completion_item).collect();

        if add_own_snippets {
            opts.extend(editor.get_snippets(id));
        }

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
            .get(&self.language)
            .and_then(Lsp::position_encoding)
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

        let lsp = editor.language_servers.get_mut(&self.language).unwrap();
        lsp.diagnostics.insert(path, diagnostics);
    }

    fn code_action(&self, editor: &mut Editor, id: ClientId, actions: Vec<CodeAction>) {
        let options: Vec<Arc<Choice>> = actions
            .iter()
            .enumerate()
            .map(|(i, action)| Choice::from_numbered_text(i + 1, action.name().to_string()))
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

        let lang = self.language.clone();
        win.prompt = Prompt::builder()
            .prompt("Select code action")
            .loads_options()
            .on_confirm(move |editor, id, out| {
                let n = getf!(out.number());
                let action = getf!(actions.get(n - 1));

                if action.is_resolved() {
                    let edit = action.workspace_edit().unwrap();
                    Self::edit_workspace(editor, id, edit)
                } else {
                    let request = RequestKind::CodeActionResolve {
                        action: action.clone().into(),
                    };

                    let lsp = getf!(editor.language_servers.get_mut(&lang));
                    let _ = lsp.request(request, id, vec![]);
                }

                ActionResult::Ok
            })
            .build();

        editor.job_broker.request(job);
    }

    fn edit_workspace(editor: &mut Editor, id: ClientId, edit: WorkspaceEdit) {
        for op in edit.file_ops {
            Self::handle_file_operation(editor, id, op);
        }
        for edit in edit.file_edits {
            Self::edit_document(editor, id, edit);
        }
    }

    fn handle_file_operation(editor: &mut Editor, id: ClientId, op: FileOperation) {
        match op {
            FileOperation::Create { path } => {
                let _ = std::fs::File::create(path);
            }
            FileOperation::Rename { from, to } => {
                handle_file_rename(editor, id, &from, &to);
            }
            FileOperation::Delete { path } => {
                handle_file_delete(editor, id, path.as_path());
            }
        }
    }

    fn edit_document(editor: &mut Editor, id: ClientId, edit: FileEdit) {
        let path = &edit.path;
        let bid = match editor.buffers().find(path) {
            Some(bid) => bid,
            None => match editor.create_buffer(id, path) {
                Ok(bid) => bid,
                Err(e) => {
                    log::error!("Failed to create buffer for {path:?} {e}");
                    return;
                }
            },
        };
        let enc = get!(editor.lsp_for(id).and_then(Lsp::position_encoding));

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
                Change::replace(start..end, edit.text.as_bytes())
            })
            .collect();
        if changes.is_empty() {
            return;
        }
        let changes = Changes::from(changes);
        let start = changes.iter().next().unwrap().start();

        match buf.apply_changes(&changes) {
            Ok(result) => {
                if let Some(id) = result.created_snapshot {
                    if let Some(aux) = buf.snapshot_additional_mut(id) {
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
        let Some(enc) = editor.lsp_for(id).and_then(Lsp::position_encoding) else {
            return;
        };

        let (win, _buf) = editor.win_buf_mut(id);
        win.locations.clear();

        locations::show_locations.execute(editor, id);

        let (win, _buf) = win_buf!(editor, id);
        win.locations.extra.title = "LSP references".to_string();

        for (path, references) in references {
            if let Some(bid) = editor.buffers.find(&path) {
                let buf = editor.buffers.get(bid).unwrap();
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
            vec![Range::from(hlstart..hlend)],
        );

        group.push(item);
    }

    group
}
