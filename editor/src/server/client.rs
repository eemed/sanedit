use std::path::PathBuf;

use tokio::{
    io,
    net::UnixStream,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::events::{FromServer, ToServer};

use super::{ServerHandle, CHANNEL_SIZE};

/// Data passed on to spawn client when acceptor accepts a new client.
pub(crate) struct ClientInfo {
    pub(crate) id: ClientId,
    pub(crate) conn: ClientConnection,
    pub(crate) server_handle: ServerHandle,
}

pub(crate) enum ClientConnection {
    UnixDomainSocket { path: PathBuf, chan: UnixStream },
    Tcp {},
}

impl ClientConnection {}

/// Client handle allows us to communicate with the client
pub(crate) struct ClientHandle {
    id: ClientId,
    send: Sender<FromServer>,
    kill: JoinHandle<()>,
}

#[derive(Clone, Copy)]
pub(crate) struct ClientId(pub(crate) usize);
pub(crate) struct Client {}

pub(crate) async fn spawn_client(info: ClientInfo) {
    let id = info.id;
    // Create a channel to receive messages from the server
    let (send, recv) = channel(CHANNEL_SIZE);

    // Create a oneshot channel to send the task the clients handle.
    // Oneshot channel is needed to get the `JoinHandle` returned by
    // tokio::spawn(run_client(..)).
    let (my_send, my_recv) = oneshot::channel();
    let kill = tokio::spawn(run_client(my_recv, recv, info));
    let handle = ClientHandle { id, send, kill };

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
    mut server_recv: Receiver<FromServer>,
    info: ClientInfo,
) -> Result<(), io::Error> {
    // Now we can
    // Send to server: info.server_handle.send()
    // Receive from server: server_recv.recv()
    //
    // TODO:
    // Send to unix domain socket: info.conn.send()
    // Read from unix domain socket: info.conn.recv()
    Ok(())
}

// /// This method performs the actual job of running the client actor.
// async fn client_loop(mut data: ClientData) -> Result<(), io::Error> {
//     let (read, write) = data.tcp.split();

//     // communication between tcp_read and tcp_write
//     let (send, recv) = unbounded_channel();

//     let ((), ()) = try_join! {
//         tcp_read(data.id, read, data.handle, send),
//         tcp_write(write, data.recv, recv),
//     }?;

//     let _ = data.tcp.shutdown().await;

//     Ok(())
// }
