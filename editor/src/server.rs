mod accept;
mod client;

pub(crate) use client::*;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::{
    net::unix::SocketAddr,
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

pub enum Address {
    UnixDomainSocket(PathBuf),
    Tcp(SocketAddr),
}

/// Run the editor.
/// Spawn connection acceptor tasks and the main editor loop task
/// The acceptor then spawns a new task for each client connection.
pub async fn run(addrs: Vec<Address>) {
    // Editor loop
    let (handle, join) = spawn_editor_loop();

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
