mod accept;
mod client;
mod job_runner;

pub(crate) use client::*;
pub(crate) use job_runner::{
    BoxedJob, FromJobs, Job, JobContext, JobId, JobResult, JobsHandle, ToJobs,
};

use std::{
    borrow::Cow,
    fmt::Display,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use tokio::{
    net::unix::SocketAddr,
    runtime::Runtime,
    sync::{
        mpsc::{channel, Sender},
        Notify,
    },
};

use crate::{editor, events::ToEditor};

use self::job_runner::spawn_jobs;

/// Channel buffer size for tokio channels
pub(crate) const CHANNEL_SIZE: usize = 64;

/// Editor handle allows us to communicate with the editor
#[derive(Clone, Debug)]
pub(crate) struct EditorHandle {
    sender: Sender<ToEditor>,
    next_id: Arc<AtomicUsize>,
}

impl EditorHandle {
    pub async fn send(&mut self, msg: ToEditor) {
        if self.sender.send(msg).await.is_err() {
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

    pub fn address_name<'a>(&'a self) -> Option<Cow<'a, str>> {
        match self {
            Address::UnixDomainSocket(p) => Some(p.as_os_str().to_string_lossy()),
            Address::Tcp(addr) => addr.as_pathname().map(|p| p.as_os_str().to_string_lossy()),
        }
    }
}

async fn listen(addrs: Vec<Address>, handle: EditorHandle) {
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

pub fn run_sync(addrs: Vec<Address>) -> Option<thread::JoinHandle<()>> {
    let (send, recv) = channel(CHANNEL_SIZE);
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };
    let rt = Runtime::new().ok()?;
    rt.block_on(listen(addrs, handle.clone()));

    thread::Builder::new()
        .name("sanedit".into())
        .spawn(move || {
            // tokio runtime is moved here, it is killed when the editor main loop exits
            let jobs_handle = rt.block_on(spawn_jobs(handle));

            if let Err(e) = editor::main_loop(jobs_handle, recv) {
                log::error!("Editor main loop exited with error {}.", e);
            }
        })
        .ok()
}
