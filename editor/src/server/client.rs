pub(crate) mod tcp;
pub(crate) mod unix;

use std::path::PathBuf;

use futures::{
    stream::{TryStream, TryStreamExt},
    Sink, SinkExt, Stream, StreamExt,
};
use sanedit_messages::{BinCodec, ClientMessage, Message};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
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
    read: impl AsyncRead,
    mut server_handle: ServerHandle,
) -> Result<(), io::Error> {
    let codec: BinCodec<Message> = BinCodec::new();
    let framed_read = FramedRead::new(read, codec);
    let mut read = Box::pin(framed_read);
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                server_handle.send(ToServer::Message(id, msg)).await;
            }
            Err(e) => {
                log::info!("conn_read error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn conn_write(
    write: impl AsyncWrite,
    mut server_recv: Receiver<FromServer>,
) -> Result<(), io::Error> {
    let codec: BinCodec<ClientMessage> = BinCodec::new();
    let mut write = Box::pin(FramedWrite::new(write, codec));

    while let Some(msg) = server_recv.recv().await {
        match msg {
            FromServer::Message(msg) => {
                if let Err(e) = write.send(msg).await {
                    log::error!("conn_write error: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
