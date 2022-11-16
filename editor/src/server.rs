mod accept;
mod client;

pub(crate) use client::*;

// TASK: Acceptor
// TASK: Each client connection proxy
// TASK: Server

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::{
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};

use crate::{editor, events::ToServer};

/// Channel buffer size for tokio channels
pub(crate) const CHANNEL_SIZE: usize = 64;

/// Editor handle allows us to communicate with the editor
#[derive(Clone, Debug)]
pub(crate) struct ServerHandle {
    sender: Sender<ToServer>,
    next_id: Arc<AtomicUsize>,
}

impl ServerHandle {
    pub async fn send(&mut self, msg: ToServer) {
        if self.sender.send(msg).await.is_err() {
            panic!("Main loop has shut down.");
        }
    }

    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}

pub enum ListenAddr {
    UnixDomainSocket(PathBuf),
    Tcp(SocketAddr),
}

pub fn run(addr: ListenAddr) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async { main(addr).await });
}

pub async fn run(addr: ListenAddr) {
    main(addr).await
}

/// Run the editor.
/// Spawn connection acceptor task and the main editor loop task
/// The acceptor then spawns a new task for each client connection.
async fn main(addr: ListenAddr) {
    // Editor loop
    let (handle, join) = spawn_editor_loop();

    // IDEA: multiple acceptors?
    tokio::spawn(async move {
        accept::accept_loop(addr, handle).await;
    });

    join.await.unwrap();
}

/// Spawn editor loop, and return a handle to it and the task join handle
fn spawn_editor_loop() -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(CHANNEL_SIZE);
    let handle = ServerHandle {
        sender: send,
        next_id: Default::default(),
    };
    let join = tokio::spawn(async move {
        let res = editor::main_loop(recv).await;
        match res {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Oops {}.", err);
            }
        }
    });

    (handle, join)
}
