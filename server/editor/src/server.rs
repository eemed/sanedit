mod accept;
mod client;

pub(crate) use client::*;

// TASK: Acceptor
// TASK: Each client connection proxy
// TASK: Server

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::{
    io,
    net::TcpListener,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};

use crate::{editor, events::ToServer};

/// Editor handle allows us to communicate with the editor
#[derive(Clone)]
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

/// Run the editor.
/// Spawn connection acceptor task and the main editor loop task
/// The acceptor then spawns a new task for each client connection.
#[tokio::main]
pub async fn run() {
    // Editor loop
    let (handle, join) = spawn_editor_loop().await;

    // Acceptor
    tokio::spawn(async move {
        // let bind = ([0, 0, 0, 0], 3456).into();
        // telnet_chat::accept::start_accept(bind, handle).await;
    });

    join.await.unwrap();
}

/// Spawn editor loop, and return a handle to it and the task join handle
async fn spawn_editor_loop() -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(64);
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
