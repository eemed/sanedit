mod accept;
mod client;
mod jobs;

pub(crate) use client::*;
pub(crate) use jobs::{
    FromJobs, Job, JobFutureFn, JobId, JobProgress, JobProgressSender, JobsHandle, PinnedFuture,
    ToJobs,
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
    sync::mpsc::{channel, Sender},
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

pub fn run_sync(addrs: Vec<Address>) {
    match Runtime::new() {
        Ok(rt) => rt.block_on(async { run(addrs).await }),
        Err(e) => log::info!("Error creating runtime: {}", e),
    }
}

/// Run the editor.
/// Spawn connection acceptor tasks and the main editor loop task
/// The acceptor then spawns a new task for each client connection.
pub async fn run(addrs: Vec<Address>) {
    let (send, recv) = channel(CHANNEL_SIZE);
    let handle = EditorHandle {
        sender: send,
        next_id: Default::default(),
    };

    let jobs_handle = spawn_jobs(handle.clone()).await;
    let join = thread::spawn(|| {
        let res = editor::main_loop(jobs_handle, recv);
        match res {
            Ok(()) => {}
            Err(err) => {
                log::error!("Oops {}.", err);
            }
        }
    });

    for addr in addrs.into_iter() {
        let h = handle.clone();
        match addr {
            Address::UnixDomainSocket(addr) => {
                tokio::spawn(async move {
                    accept::unix::accept_loop(addr, h).await;
                });
            }
            Address::Tcp(addr) => {
                tokio::spawn(async move {
                    accept::tcp::accept_loop(addr, h).await;
                });
            }
        }
    }

    join.join().unwrap();
}

// /// Spawn editor loop, and return a handle to it and the task join handle
// fn spawn_editor_loop(jobs_handle: JobsHandle) -> (ServerHandle, std::thread::JoinHandle<()>) {

//     (handle, join)
// }
