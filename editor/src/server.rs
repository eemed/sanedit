mod accept;
mod client;
mod jobs;

pub(crate) use client::*;
pub(crate) use jobs::{
    FromJobs, Job, JobFutureFn, JobId, JobProgress, JobProgressSender, JobsHandle,
};

use std::{
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

use self::jobs::spawn_jobs;

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

pub(crate) struct Setup {
    pub jobs_handle: JobsHandle,
}

async fn setup(addrs: Vec<Address>, handle: EditorHandle) -> Setup {
    for addr in addrs.into_iter() {
        let notify = Arc::new(Notify::new());
        let n = notify.clone();
        let h = handle.clone();
        match addr {
            Address::UnixDomainSocket(addr) => {
                tokio::spawn(async move {
                    accept::unix::accept_loop(addr, h, n).await;
                });
            }
            Address::Tcp(addr) => {
                tokio::spawn(async move {
                    accept::tcp::accept_loop(addr, h, n).await;
                });
            }
        }

        notify.notified().await;
    }

    let jobs_handle = spawn_jobs(handle).await;

    Setup { jobs_handle }
}

pub fn run_sync(addrs: Vec<Address>) -> Option<thread::JoinHandle<()>> {
    let (send, recv) = channel(CHANNEL_SIZE);
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };
    let cloned = handle.clone();
    let rt = Runtime::new().ok()?;
    let setup = rt.block_on(async move { setup(addrs, cloned).await });

    let join = thread::spawn(move || {
        let _rt = rt;
        if let Err(e) = editor::main_loop(setup, recv) {
            log::error!("Oops {}.", e);
        }
    });

    Some(join)
}
