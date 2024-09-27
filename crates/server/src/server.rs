pub(crate) mod accept;
pub(crate) mod client;

use std::{
    borrow::Cow,
    fmt::Display,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::Sender,
        Arc,
    },
};

use tokio::{net::unix::SocketAddr, sync::Notify};

use crate::events::ToEditor;

use self::client::ClientId;

#[derive(Clone, Debug)]
pub struct StartOptions {
    pub open_files: Vec<PathBuf>,
    pub config_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
}

/// Editor handle allows us to communicate with the editor
#[derive(Clone, Debug)]
pub struct EditorHandle {
    pub sender: Sender<ToEditor>,
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

pub enum Address {
    UnixDomainSocket(PathBuf),
    Tcp(SocketAddr),
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
            Address::Tcp(_) => "tcp",
        }
    }

    pub fn address_name(&self) -> Option<Cow<str>> {
        match self {
            Address::UnixDomainSocket(p) => Some(p.as_os_str().to_string_lossy()),
            Address::Tcp(addr) => addr.as_pathname().map(|p| p.as_os_str().to_string_lossy()),
        }
    }
}

pub async fn spawn_listeners(addrs: Vec<Address>, handle: EditorHandle) {
    for addr in addrs.into_iter() {
        let addr_ready = Arc::new(Notify::new());

        let n = addr_ready.clone();
        let h = handle.clone();
        let display = format!("{}", addr);

        match addr {
            Address::UnixDomainSocket(addr) => {
                tokio::spawn(accept::unix::accept_loop(addr.clone(), h, n));
            }
            Address::Tcp(addr) => {
                tokio::spawn(accept::tcp::accept_loop(addr, h, n));
            }
        }

        addr_ready.notified().await;
        log::info!("Server listening at {display}");
    }
}
