use std::{
    any::Any,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use crate::{
    actions::{
        hooks::run,
        locations,
        lsp::{self, position_to_offset, range_to_buffer_range},
    },
    editor::{
        buffers::BufferId,
        hooks::Hook,
        job_broker::KeepInTouch,
        options::LSPOptions,
        windows::{Completion, Prompt},
        Editor, Map,
    },
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};
use sanedit_buffer::{PieceTree, PieceTreeSlice};
use sanedit_core::{Change, Changes, Filetype, Group, Item, MatchOption, MatchStrategy};
use sanedit_lsp::{
    lsp_types::{self, CodeAction, Position},
    CompletionItem, LSPClientParams, LSPClientSender, Notification, Reference, Request,
    RequestKind, RequestResult, Response,
};

use anyhow::Result;
use sanedit_messages::redraw::{Severity, StatusMessage};

use super::MatcherJob;

/// A way to discard non relevant LSP reponses.
/// For example if we complete a completion request when the cursor has already
/// moved, there is no point anymore.
#[derive(Debug)]
pub(crate) enum Constraint {
    Buffer(BufferId),
    BufferVersion(u32),
    CursorPosition(u64),
}

/// A handle to send operations to LSP instance.
///
/// LSP is running in a job slot and communicates back using messages.
///
#[derive(Debug)]
pub(crate) struct LSPHandle {
    /// Name of the LSP server
    name: String,

    /// Root where LSP is started
    root: PathBuf,

    /// Client to send messages to LSP server
    sender: LSPClientSender,

    /// Constraints that need to be met in order to execute request responses
    requests: Map<u32, (ClientId, Vec<Constraint>)>,

    request_id: AtomicU32,
}

impl LSPHandle {
    pub fn server_name(&self) -> &str {
        &self.name
    }

    pub fn next_id(&self) -> u32 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn reponse_of(&mut self, id: u32) -> Option<(ClientId, Vec<Constraint>)> {
        self.requests.remove(&id)
    }

    pub fn request(
        &mut self,
        req: RequestKind,
        cid: ClientId,
        constraints: Vec<Constraint>,
    ) -> Result<()> {
        let id = self.next_id();
        self.requests.insert(id, (cid, constraints));
        self.sender.request(Request { id, kind: req });
        Ok(())
    }

    pub fn notify(&mut self, op: Notification) -> Result<()> {
        self.sender.notify(op);
        Ok(())
    }

    pub fn init_params(&self) -> &lsp_types::InitializeResult {
        self.sender.init_params()
    }

    pub fn position_encoding(&self) -> lsp_types::PositionEncodingKind {
        self.sender
            .init_params()
            .capabilities
            .position_encoding
            .clone()
            .unwrap_or(lsp_types::PositionEncodingKind::UTF16)
    }
}

#[derive(Clone)]
pub(crate) struct LSP {
    client_id: ClientId,
    filetype: Filetype,
    working_dir: PathBuf,
    opts: LSPOptions,
}

impl LSP {
    pub fn new(id: ClientId, working_dir: PathBuf, ft: Filetype, opts: &LSPOptions) -> LSP {
        LSP {
            client_id: id,
            filetype: ft,
            working_dir,
            opts: opts.clone(),
        }
    }
}

impl Job for LSP {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        // Clones here
        let wd = self.working_dir.clone();
        let ft = self.filetype.clone();
        let opts = self.opts.clone();

        let fut = async move {
            log::info!("Run rust-analyzer");
            let LSPOptions { command, args } = opts;
            let filetype: String = ft.as_str().into();
            let params = LSPClientParams {
                run_command: command.clone(),
                run_args: args,
                root: wd.clone(),
                filetype,
            };

            let (sender, mut reader) = params.spawn().await?;
            log::info!("Client started");

            let handle = LSPHandle {
                name: command,
                sender,
                root: wd,
                requests: Map::default(),
                request_id: AtomicU32::new(1),
            };
            ctx.send(Message::Started(handle));

            while let Some(response) = reader.recv().await {
                ctx.send(Message::Response(response));
            }

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for LSP {
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
                        lsp::open_document(editor, id);
                    }
                }
                Message::Response(response) => self.handle_response(editor, response),
            }
        }
    }
}

impl LSP {
    fn handle_response(&self, editor: &mut Editor, response: Response) {
        match response {
            Response::Request { id, result } => {
                let Some(lsp) = editor.language_servers.get_mut(&self.filetype) else {
                    return;
                };
                let Some((id, constraints)) = lsp.reponse_of(id) else {
                    return;
                };

                // Verify constraints
                let (win, buf) = editor.win_buf(id);
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

                self.handle_result(editor, id, result);
            }
            Response::Notification(_) => todo!(),
        }
    }

    fn handle_result(&self, editor: &mut Editor, id: ClientId, result: RequestResult) {
        match result {
            RequestResult::Hover { text, position } => {
                let (win, buf) = editor.win_buf_mut(id);
                win.popup = Some(StatusMessage {
                    severity: Severity::Info,
                    message: text,
                });
            }
            RequestResult::GotoDefinition { path, position } => {
                if editor.open_file(self.client_id, path).is_ok() {
                    let enc = editor
                        .lsp_handle_for(self.client_id)
                        .map(|x| x.position_encoding());
                    if let Some(enc) = enc {
                        let (win, buf) = editor.win_buf_mut(self.client_id);
                        let slice = buf.slice(..);
                        let offset = position_to_offset(&slice, position, &enc);
                        win.goto_offset(offset, buf);
                    }
                }
            }
            RequestResult::Complete {
                path,
                position,
                results,
            } => complete(editor, id, path, position, results),
            RequestResult::References { references } => show_references(editor, id, references),
            RequestResult::CodeAction { actions } => code_action(editor, id, actions),
            RequestResult::ResolvedAction { action } => {
                if let Some(edit) = action.edit {
                    edit_workspace(editor, id, edit)
                }
            }
            RequestResult::Rename { edit } => edit_workspace(editor, id, edit),
            RequestResult::Error { msg } => {
                log::error!("Got error message from LSP {}: {msg}", self.opts.command);
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Started(LSPHandle),
    Response(Response),
}

fn complete(
    editor: &mut Editor,
    id: ClientId,
    path: PathBuf,
    position: Position,
    opts: Vec<CompletionItem>,
) {
    let Some(enc) = editor.lsp_handle_for(id).map(|x| x.position_encoding()) else {
        return;
    };
    let (win, buf) = editor.win_buf_mut(id);
    let slice = buf.slice(..);
    let start = position_to_offset(&slice, position, &enc);
    win.completion = Completion::new(start);

    let cursor = win.primary_cursor();
    if let Some(point) = win.view().point_at_pos(cursor.pos()) {
        win.completion.point = point;
    }

    let opts: Vec<MatchOption> = opts.into_iter().map(from_completion_item).collect();

    let job = MatcherJob::builder(id)
        .strategy(MatchStrategy::Prefix)
        .options(Arc::new(opts))
        .handler(Completion::matcher_result_handler)
        .build();

    editor.job_broker.request(job);
}

fn code_action(editor: &mut Editor, id: ClientId, actions: Vec<CodeAction>) {
    let options: Vec<String> = actions.iter().map(|a| a.title.clone()).collect();
    let (win, _buf) = editor.win_buf_mut(id);

    let job = MatcherJob::builder(id)
        .options(Arc::new(options))
        .handler(Prompt::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Select code action")
        .on_confirm(move |editor, id, input| {
            let (win, buf) = editor.win_buf_mut(id);
            let Some(ft) = buf.filetype.clone() else {
                return;
            };

            let Some(action) = actions.iter().find(|action| action.title == input) else {
                return;
            };

            let resolved = action.edit.is_some();
            if !resolved {
                let request = RequestKind::CodeActionResolve {
                    action: action.clone(),
                };

                let Some(lsp) = editor.language_servers.get_mut(&ft) else {
                    return;
                };
                let _ = lsp.request(request, id, vec![]);
                return;
            }
        })
        .build();

    editor.job_broker.request(job);
}

fn edit_workspace(editor: &mut Editor, id: ClientId, edit: lsp_types::WorkspaceEdit) {
    let lsp_types::WorkspaceEdit {
        changes,
        document_changes,
        change_annotations,
    } = edit;

    if let Some(doc_changes) = document_changes {
        match doc_changes {
            lsp_types::DocumentChanges::Edits(edits) => {
                for edit in edits {
                    edit_document(editor, id, edit);
                }
            }
            lsp_types::DocumentChanges::Operations(ops) => {
                for op in ops {
                    match op {
                        lsp_types::DocumentChangeOperation::Op(op) => {
                            code_action_edit_operation(editor, id, op);
                        }
                        lsp_types::DocumentChangeOperation::Edit(edit) => {
                            edit_document(editor, id, edit);
                        }
                    }
                }
            }
        }
    }
}

fn code_action_edit_operation(editor: &mut Editor, id: ClientId, op: lsp_types::ResourceOp) {
    match op {
        lsp_types::ResourceOp::Create(create) => todo!(),
        lsp_types::ResourceOp::Rename(rename) => todo!(),
        lsp_types::ResourceOp::Delete(delete) => todo!(),
    }
}

fn edit_document(editor: &mut Editor, id: ClientId, edit: lsp_types::TextDocumentEdit) {
    let path = PathBuf::from(edit.text_document.uri.path().as_str());
    let bid = match editor.buffers().find(&path) {
        Some(bid) => bid,
        None => match editor.create_buffer(id, &path) {
            Ok(bid) => bid,
            Err(_) => {
                log::error!("Failed to create buffer for {path:?}");
                return;
            }
        },
    };
    let Some(enc) = editor.lsp_handle_for(id).map(|x| x.position_encoding()) else {
        return;
    };

    // The changes build on themselves so apply each separately
    for (i, edit) in edit.edits.into_iter().enumerate() {
        let buf = editor.buffers_mut().get_mut(bid).unwrap();
        let slice = buf.slice(..);
        let text_edit = {
            match edit {
                lsp_types::OneOf::Left(item) => item,
                lsp_types::OneOf::Right(item) => item.text_edit,
            }
        };

        let range = range_to_buffer_range(&slice, text_edit.range, &enc);
        let change = Change::replace(range, text_edit.new_text.as_bytes());
        let mut changes = Changes::from(change);
        // Disable undo point creation for other than first change
        if i != 0 {
            changes.disable_undo_point_creation();
        }

        if let Err(e) = buf.apply_changes(&changes) {
            log::error!("Failed to apply changes to buffer: {path:?}: {e}");
            return;
        }

        run(editor, id, Hook::BufChanged(bid));
    }
}

fn show_references(
    editor: &mut Editor,
    id: ClientId,
    references: BTreeMap<PathBuf, Vec<Reference>>,
) {
    let Some(enc) = editor
        .lsp_handle_for(id)
        .map(|handle| handle.position_encoding())
    else {
        return;
    };

    // TODO should this be auto shown?
    locations::show.execute(editor, id);

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

fn read_references(
    slice: &PieceTreeSlice,
    path: &Path,
    references: &[Reference],
    enc: &lsp_types::PositionEncodingKind,
) -> Group {
    let mut group = Group::new(&path);

    for re in references {
        let start = position_to_offset(&slice, re.start, enc);
        let end = position_to_offset(&slice, re.end, enc);
        let (row, line) = slice.line_at(start);
        let lstart = line.start();

        let hlstart = (start - lstart) as usize;
        let hlend = (end - lstart) as usize;
        let text = String::from(&line);
        let text = text.trim_end();
        let item = Item::new(text, Some(row), Some(lstart), vec![hlstart..hlend]);

        group.push(item);
    }

    group
}

fn from_completion_item(value: CompletionItem) -> MatchOption {
    match value.description {
        Some(desc) => MatchOption::with_description(&value.name, &desc),
        None => MatchOption::from(value.name),
    }
}
