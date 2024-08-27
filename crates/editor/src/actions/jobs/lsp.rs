use std::{
    any::Any,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    actions::lsp::{self, position_to_offset},
    common::matcher::{Kind, MatchOption, MatchStrategy},
    editor::{
        buffers::{Buffer, BufferId, Filetype},
        job_broker::KeepInTouch,
        options::LSPOptions,
        windows::{Completion, Focus, Group, Item},
        Editor,
    },
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};
use sanedit_buffer::{PieceTree, PieceTreeSlice};
use sanedit_lsp::{
    lsp_types::{self, Position},
    CompletionItem, LSPClientParams, LSPClientSender, Reference, Request, Response,
};

use anyhow::Result;
use sanedit_messages::redraw::{Severity, StatusMessage};

use super::MatcherJob;

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
}

impl LSPHandle {
    pub fn server_name(&self) -> &str {
        &self.name
    }

    pub fn send(&mut self, op: Request) -> Result<()> {
        self.sender.send(op);
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

            let sender = LSPHandle {
                name: command,
                sender,
                root: wd,
            };
            ctx.send(Message::Started(ft, sender));

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
        let (win, buf) = editor.win_buf_mut(self.client_id);

        if let Ok(output) = msg.downcast::<Message>() {
            match *output {
                Message::Started(ft, sender) => {
                    // Set sender
                    editor.language_servers.insert(ft, sender);

                    // Send all buffers of this filetype
                    let ids: Vec<BufferId> = editor.buffers().iter().map(|(id, _buf)| id).collect();
                    for id in ids {
                        lsp::open_document(editor, id);
                    }
                }
                Message::Response(response) => match response {
                    Response::Request(request) => match request {
                        sanedit_lsp::RequestResult::Hover { text, position } => {
                            win.popup = Some(StatusMessage {
                                severity: Severity::Info,
                                message: text,
                            });
                        }
                        sanedit_lsp::RequestResult::GotoDefinition { path, position } => {
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
                        sanedit_lsp::RequestResult::Complete {
                            path,
                            position,
                            results,
                        } => complete(editor, self.client_id, path, position, results),
                        sanedit_lsp::RequestResult::References { references } => {
                            show_references(editor, self.client_id, references)
                        }
                    },
                },
            }
        }
    }
}

#[derive(Debug)]
enum Message {
    Started(Filetype, LSPHandle),
    Response(Response),
}

fn complete(
    editor: &mut Editor,
    id: ClientId,
    path: PathBuf,
    position: Position,
    opts: Vec<CompletionItem>,
) {
    // TODO how to ensure this is not old data

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

    let opts: Vec<MatchOption> = opts.into_iter().map(MatchOption::from).collect();

    let job = MatcherJob::builder(id)
        .strategy(MatchStrategy::Prefix)
        .options(Arc::new(opts))
        .handler(Completion::matcher_result_handler)
        .build();

    editor.job_broker.request(job);
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

    let (win, buf) = editor.win_buf_mut(id);
    win.locations.clear();
    win.locations.show = true;
    win.focus = Focus::Locations;

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
        let item = Item::new(&text, Some(row), Some(start), vec![hlstart..hlend]);

        log::info!("Item: {item:?}");
        group.push(item);
    }

    group
}

impl From<CompletionItem> for MatchOption {
    fn from(value: CompletionItem) -> Self {
        MatchOption::from(value.name)
    }
}
