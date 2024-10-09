use std::{
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use sanedit_core::{Diagnostic, Filetype};
use sanedit_lsp::{
    LSPClientSender, LSPRequestError, Notification, PositionEncoding, Request, RequestKind,
};
use sanedit_server::ClientId;
use sanedit_utils::sorted_vec::SortedVec;

use super::{
    buffers::{Buffer, BufferId},
    Map,
};

pub(crate) fn get_diagnostics<'a>(
    buf: &Buffer,
    language_servers: &'a Map<Filetype, LSP>,
) -> Option<&'a [Diagnostic]> {
    let ft = buf.filetype.as_ref()?;
    let path = buf.path()?;
    let lsp = language_servers.get(ft)?;
    let diags = lsp.diagnostics.get(path)?;
    Some(diags)
}

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
pub(crate) struct LSP {
    /// Name of the LSP server
    name: String,

    /// Client to send messages to LSP server
    sender: LSPClientSender,

    /// Constraints that need to be met in order to execute request responses
    requests: Map<u32, (ClientId, Vec<Constraint>)>,

    request_id: AtomicU32,

    /// Diagnostics per file
    pub diagnostics: Map<PathBuf, SortedVec<Diagnostic>>,
}

impl LSP {
    pub fn new(name: &str, sender: LSPClientSender) -> LSP {
        LSP {
            name: name.into(),
            sender,
            requests: Map::default(),
            request_id: AtomicU32::new(1),
            diagnostics: Map::default(),
        }
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
        self.sender.request(Request { id, kind: req })?;
        Ok(())
    }

    pub fn notify(&mut self, op: Notification) -> Result<(), LSPRequestError> {
        self.sender.notify(op)?;
        Ok(())
    }

    pub fn position_encoding(&self) -> PositionEncoding {
        self.sender.position_encoding()
    }
}
