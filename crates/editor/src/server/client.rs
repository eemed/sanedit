mod draw;
pub(crate) mod tcp;
pub(crate) mod unix;

use std::path::PathBuf;

use futures::{SinkExt, StreamExt};
use sanedit_messages::{BinCodec, ClientMessage, Message};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::events::{FromEditor, ToEditor};

use self::draw::ClientDrawState;

use super::EditorHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ClientId(pub(crate) usize);

/// Client handle allows us to communicate with the client
#[derive(Debug)]
pub(crate) struct ClientHandle {
    pub(crate) id: ClientId,
    pub(crate) info: ClientConnectionInfo,
    pub(crate) send: Sender<FromEditor>,
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
    mut server_handle: EditorHandle,
) -> Result<(), io::Error> {
    let codec: BinCodec<Message> = BinCodec::new();
    let framed_read = FramedRead::new(read, codec);
    let mut read = Box::pin(framed_read);
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                server_handle.send(ToEditor::Message(id, msg)).await;
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
    mut server_recv: Receiver<FromEditor>,
) -> Result<(), io::Error> {
    let mut state = ClientDrawState::default();
    let codec: BinCodec<ClientMessage> = BinCodec::new();
    let mut write = Box::pin(FramedWrite::new(write, codec));

    while let Some(msg) = server_recv.recv().await {
        match msg {
            FromEditor::Message(mut msg) => {
                // TODO move diffing away from here? This basically acts as a
                // middleware to diff changes before they are sent to the
                // client. Diffing is done here so editor can just send stuff
                // without diffing all the changes itself.
                if let ClientMessage::Redraw(redraw) = msg {
                    match state.handle_redraw(redraw) {
                        Some(new_redraw) => msg = new_redraw.into(),
                        None => continue,
                    }
                }

                if let Err(e) = write.send(msg).await {
                    log::error!("conn_write error: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
