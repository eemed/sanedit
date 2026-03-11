use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use sanedit_core::Diagnostic;
use sanedit_lsp::{
    LSPClientSender, LSPRequestError, Notification, PositionEncoding, Request, RequestKind,
};
use sanedit_server::ClientId;
use sanedit_utils::sorted_vec::SortedVec;

use super::{
    buffers::{Buffer, BufferId},
    Map,
};

/// A way to discard non relevant LSP reponses.
/// For example if we complete a completion request when the cursor has already
/// moved, there is no point anymore.
#[derive(Debug)]
pub(crate) enum Constraint {
    Buffer(BufferId),
    BufferVersion(u32),
    CursorPosition(u64),
}

#[derive(Debug)]
enum DiagnosticList {
    Resolved(SortedVec<Diagnostic>),
    Unresolved(Vec<sanedit_lsp::TextDiagnostic>),
}

/// A handle to send operations to LSP instance.
///
/// LSP is running in a job slot and communicates back using messages.
///
#[derive(Debug)]
pub(crate) struct Lsp {
    /// Name of the LSP server
    name: String,

    /// Client to send messages to LSP server
    sender: Option<LSPClientSender>,

    /// Constraints that need to be met in order to execute request responses
    requests: Map<u32, (ClientId, Vec<Constraint>)>,

    request_id: AtomicU32,

    /// Diagnostics per file
    diagnostics: Map<PathBuf, DiagnosticList>,
}

impl Lsp {
    pub fn new(name: &str) -> Lsp {
        Lsp {
            name: name.into(),
            sender: None,
            requests: Map::default(),
            request_id: AtomicU32::new(1),
            diagnostics: Map::default(),
        }
    }

    pub fn start(&mut self, sender: LSPClientSender) {
        self.sender = Some(sender);
    }

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
    ) -> Result<(), LSPRequestError> {
        let id = self.next_id();
        self.requests.insert(id, (cid, constraints));
        let sender = self
            .sender
            .as_mut()
            .ok_or(LSPRequestError::ServerNotStarted)?;
        sender.request(Request { id, kind: req })?;
        Ok(())
    }

    pub fn notify(&mut self, op: Notification) -> Result<(), LSPRequestError> {
        let sender = self
            .sender
            .as_mut()
            .ok_or(LSPRequestError::ServerNotStarted)?;
        sender.notify(op)?;
        Ok(())
    }

    pub fn position_encoding(&self) -> Option<PositionEncoding> {
        let sender = self.sender.as_ref()?;
        Some(sender.position_encoding())
    }

    pub fn inflight_requests(&self, client: ClientId) -> usize {
        self.requests
            .iter()
            .filter(|(_, (id, _))| id == &client)
            .count()
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    pub fn add_diagnostics(&mut self, path: &Path, diags: Vec<sanedit_lsp::TextDiagnostic>) {
        self.diagnostics
            .insert(path.to_path_buf(), DiagnosticList::Unresolved(diags));
    }

    pub fn diagnostics(&mut self, buf: &Buffer) -> Option<&[Diagnostic]> {
        let path = buf.path()?;
        let enc = self.position_encoding()?;
        let diagnostics = self.diagnostics.get_mut(path)?;
        if let DiagnosticList::Unresolved(text_diagnostics) = diagnostics {
            let slice = buf.slice(..);
            let converted_diags = text_diagnostics
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

            *diagnostics = DiagnosticList::Resolved(converted_diags);
        }

        if let DiagnosticList::Resolved(diags) = diagnostics {
            Some(diags.iter().as_slice())
        } else {
            None
        }
    }
}
