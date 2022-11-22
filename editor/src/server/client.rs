pub(crate) mod tcp;
pub(crate) mod unix;

use std::{path::PathBuf, pin::Pin};

use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::events::FromServer;

use super::ServerHandle;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientId(pub(crate) usize);

/// Client handle allows us to communicate with the client
#[derive(Debug)]
pub(crate) struct ClientHandle {
    pub(crate) id: ClientId,
    pub(crate) info: ClientConnectionInfo,
    pub(crate) send: Sender<FromServer>,
    pub(crate) kill: JoinHandle<()>,
}

/// Information on how the client is connected
#[derive(Debug, Clone)]
pub(crate) enum ClientConnectionInfo {
    UnixDomainSocket(PathBuf),
    Tcp(),
}

async fn conn_read(
    id: ClientId,
    read: impl AsyncReadExt,
    server_handle: ServerHandle,
) -> Result<(), io::Error> {
    // let mut read = Box::pin(read);
    // loop {
    //     let mut buf = [0u8; 256];
    //     let size = read.read(&mut buf).await.unwrap();
    //     println!("BUF: {:?}", &buf[..size]);
    // }
    Ok(())
}

async fn conn_write(
    write: impl AsyncWriteExt,
    server_recv: Receiver<FromServer>,
) -> Result<(), io::Error> {
    Ok(())
}
