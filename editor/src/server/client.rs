use tokio::sync::mpsc::Sender;

use crate::events::FromServer;

pub(crate) struct ClientId(pub(crate) usize);

/// Client handle allows us to communicate with the client
pub(crate) struct ClientHandle {
    send: Sender<FromServer>,
}
pub(crate) struct Client {}

pub(crate) async fn spawn_client() {
    todo!()
}

// pub async fn spawn_client() {
//     let (send, recv) = channel(64);

//     let data = ClientData {
//         id: info.id,
//         handle: info.handle.clone(),
//         tcp: info.tcp,
//         recv,
//     };

//     // This spawns the new task.
//     let (my_send, my_recv) = oneshot::channel();
//     let kill = tokio::spawn(start_client(my_recv, data));

//     // Then we create a ClientHandle to this new task, and use the oneshot
//     // channel to send it to the task.
//     let handle = ClientHandle {
//         id: info.id,
//         ip: info.ip,
//         chan: send,
//         kill,
//     };

//     // Ignore send errors here. Should only happen if the server is shutting
//     // down.
//     let _ = my_send.send(handle);
// }
// async fn start_client(my_handle: oneshot::Receiver<ClientHandle>, mut data: ClientData) {
//     // Wait for `spawn_client` to send us the `ClientHandle` so we can forward
//     // it to the main loop. We need the oneshot channel because we cannot
//     // otherwise get the `JoinHandle` returned by `tokio::spawn`. We forward it
//     // from here instead of in `spawn_client` because we want the server to see
//     // the NewClient message before this actor starts sending other messages.
//     let my_handle = match my_handle.await {
//         Ok(my_handle) => my_handle,
//         Err(_) => return,
//     };
//     data.handle.send(ToServer::NewClient(my_handle)).await;

//     // We sent the client handle to the main loop. Start talking to the tcp
//     // connection.
//     let res = client_loop(data).await;
//     match res {
//         Ok(()) => {},
//         Err(err) => {
//             eprintln!("Something went wrong: {}.", err);
//         },
//     }
// }

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
