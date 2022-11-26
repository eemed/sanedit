pub(crate) mod tcp;
pub(crate) mod unix;

use std::{path::PathBuf, pin::Pin};

use futures::{SinkExt, StreamExt};
use sanedit_messages::{BinCodec, ClientMessage, Decoder, Message};
use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::events::{FromServer, ToServer};

use super::ServerHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    mut server_handle: ServerHandle,
) -> Result<(), io::Error> {
    let codec: BinCodec<Message> = BinCodec::new();
    let mut read = Box::pin(FramedRead::new(read, codec));
    for msg in read.next().await {
        match msg {
            Ok(msg) => {
                server_handle.send(ToServer::Message(id, msg)).await;
            }
            Err(e) => {
                println!("conn_read error: {}", e);
            }
        }
    }
    Ok(())
}

async fn conn_write(
    write: impl AsyncWriteExt,
    mut server_recv: Receiver<FromServer>,
) -> Result<(), io::Error> {
    let codec: BinCodec<ClientMessage> = BinCodec::new();
    let mut write = Box::pin(FramedWrite::new(write, codec));

    while let Some(msg) = server_recv.recv().await {
        match msg {
            FromServer::Message(msg) => {
                if let Err(e) = write.send(msg).await {
                    println!("conn_write error: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
