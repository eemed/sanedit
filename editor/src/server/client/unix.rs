use std::path::PathBuf;

use tokio::{
    io::{self, AsyncWriteExt},
    net::UnixStream,
    sync::{
        mpsc::{channel, Receiver},
        oneshot,
    },
    try_join,
};

use crate::{
    events::{FromServer, ToServer},
    server::{
        client::{conn_read, conn_write},
        ServerHandle, CHANNEL_SIZE,
    },
};

use super::{ClientConnectionInfo, ClientHandle, ClientId};

/// Data passed on to spawn client when acceptor accepts a new client.
#[derive(Debug)]
pub(crate) struct ClientInfo {
    pub(crate) id: ClientId,
    pub(crate) conn: UnixStream,
    pub(crate) path: PathBuf,
    pub(crate) server_handle: ServerHandle,
}

pub(crate) fn spawn_client(info: ClientInfo) {
    let id = info.id;
    let path = info.path.clone();
    // Create a channel to receive messages from the server
    let (send, recv) = channel(CHANNEL_SIZE);

    // Create a oneshot channel to send the task the clients handle.
    // Oneshot channel is needed to get the `JoinHandle` returned by
    // tokio::spawn(run_client(..)).
    // It also needs to be sent from the run_client function because otherwise
    // the server could receive messages before we send the client handle in a
    // NewClient event.
    let (my_send, my_recv) = oneshot::channel();
    let kill = tokio::spawn(run_client(my_recv, recv, info));
    let handle = ClientHandle {
        id,
        send,
        info: ClientConnectionInfo::UnixDomainSocket(path),
        kill,
    };

    // Ignore send errors here. Should only happen if the server is shutting
    // down.
    let _ = my_send.send(handle);
}

async fn run_client(
    my_handle: oneshot::Receiver<ClientHandle>,
    server_recv: Receiver<FromServer>,
    mut info: ClientInfo,
) {
    let my_handle = match my_handle.await {
        Ok(my_handle) => my_handle,
        Err(_) => return,
    };

    // Send client handle to the server
    info.server_handle
        .send(ToServer::NewClient(my_handle))
        .await;

    let res = client_loop(server_recv, info).await;
    match res {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Something went wrong: {}.", err);
        }
    }
}

async fn client_loop(
    server_recv: Receiver<FromServer>,
    mut info: ClientInfo,
) -> Result<(), io::Error> {
    let (read, write) = info.conn.split();

    let ((), ()) = try_join! {
        conn_read(info.id, read, info.server_handle),
        conn_write(write, server_recv),
    }?;

    let _ = info.conn.shutdown().await;
    Ok(())
}
