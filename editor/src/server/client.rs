use std::path::PathBuf;

use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
    try_join,
};

use crate::events::{FromServer, ToServer};

use super::{ServerHandle, CHANNEL_SIZE};

/// Data passed on to spawn client when acceptor accepts a new client.
#[derive(Debug)]
pub(crate) struct ClientInfo {
    pub(crate) id: ClientId,
    pub(crate) conn: ClientConnection,
    pub(crate) server_handle: ServerHandle,
}

/// Information on how the client is connected
#[derive(Debug, Clone)]
pub(crate) enum ConnectionInfo {
    UnixDomainSocket(PathBuf),
    Tcp {},
}

#[derive(Debug)]
pub(crate) enum Connection {
    UnixDomainSocket(UnixStream),
    Tcp {},
}

#[derive(Debug)]
pub(crate) struct ClientConnection {
    info: ConnectionInfo,
    conn: Connection,
}

impl ClientConnection {
    pub(crate) fn from_unix_domain_socket(chan: UnixStream, socket: PathBuf) -> ClientConnection {
        ClientConnection {
            info: ConnectionInfo::UnixDomainSocket(socket),
            conn: Connection::UnixDomainSocket(chan),
        }
    }

    fn split<'a>(&'a mut self) -> (impl AsyncReadExt + 'a, impl AsyncWriteExt + 'a) {
        match &mut self.conn {
            Connection::UnixDomainSocket(chan) => chan.split(),
            Connection::Tcp {} => todo!(),
        }
    }

    async fn shutdown(&mut self) -> Result<(), io::Error> {
        match &mut self.conn {
            Connection::UnixDomainSocket(chan) => chan.shutdown().await,
            Connection::Tcp {} => todo!(),
        }
    }
}

/// Client handle allows us to communicate with the client
#[derive(Debug)]
pub(crate) struct ClientHandle {
    id: ClientId,
    conn_info: ConnectionInfo,
    send: Sender<FromServer>,
    kill: JoinHandle<()>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientId(pub(crate) usize);
pub(crate) struct Client {}

pub(crate) fn spawn_client(info: ClientInfo) {
    let id = info.id;
    let conn_info = info.conn.info.clone();
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
        conn_info,
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

async fn conn_read(
    id: ClientId,
    read: impl AsyncReadExt,
    server_handle: ServerHandle,
) -> Result<(), io::Error> {
    todo!()
}

async fn conn_write(
    write: impl AsyncWriteExt,
    server_recv: Receiver<FromServer>,
) -> Result<(), io::Error> {
    todo!()
}
