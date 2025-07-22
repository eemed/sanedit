pub(crate) mod accept;
pub(crate) mod client;

use std::{
    borrow::Cow,
    fmt::Display,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::sync::Notify;

use crate::events::ToEditor;

use self::client::ClientId;

#[derive(Clone, Debug)]
pub struct StartOptions {
    pub config_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub debug: bool,
    pub addr: Address,
}

/// Editor handle allows us to communicate with the editor
#[derive(Clone, Debug)]
pub struct EditorHandle {
    pub sender: crossbeam::channel::Sender<ToEditor>,
    pub next_id: Arc<AtomicUsize>,
}

impl EditorHandle {
    pub fn send(&mut self, msg: ToEditor) {
        if self.sender.send(msg).is_err() {
            panic!("Main loop has shut down.");
        }
    }

    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}

#[derive(Debug, Clone)]
pub enum Address {
    UnixDomainSocket(PathBuf),
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prot = self.protocol();
        f.write_str(prot)?;
        if let Some(name) = self.address_name() {
            f.write_fmt(format_args!("@{}", name))?;
        }
        Ok(())
    }
}

impl Address {
    pub fn protocol(&self) -> &str {
        match self {
            Address::UnixDomainSocket(_) => "unix-domain-socket",
        }
    }

    pub fn address_name(&self) -> Option<Cow<str>> {
        match self {
            Address::UnixDomainSocket(p) => Some(p.as_os_str().to_string_lossy()),
        }
    }

    /// Return address in a way that can be used with sane --connect <addr>
    pub fn as_connect(&self) -> String {
        match self {
            Address::UnixDomainSocket(p) => p.as_os_str().to_string_lossy().to_string(),
        }
    }
}

pub async fn spawn_listener(addr: Address, handle: EditorHandle) {
    let addr_ready = Arc::new(Notify::new());

    let n = addr_ready.clone();
    let h = handle.clone();
    let display = format!("{}", addr);

    match addr {
        Address::UnixDomainSocket(addr) => {
            tokio::spawn(accept::unix::accept_loop(addr.clone(), h, n));
        }
    }

    addr_ready.notified().await;
    log::info!("Server listening at {display}");
}
