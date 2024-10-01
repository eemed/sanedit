mod draw;
pub(crate) mod tcp;
pub(crate) mod unix;

use std::{borrow::Cow, path::PathBuf};

use futures_util::sink::SinkExt;
use futures_util::StreamExt;
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
pub struct ClientId(pub usize);

impl From<ClientId> for String {
    fn from(value: ClientId) -> Self {
        value.0.to_string()
    }
}

/// Client handle allows us to communicate with the client
#[derive(Debug)]
pub struct ClientHandle {
    pub(crate) id: ClientId,
    pub(crate) info: ClientConnectionInfo,
    pub(crate) send: Sender<FromEditor>,
    pub(crate) _kill: JoinHandle<()>,
}

impl ClientHandle {
    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn connection_info(&self) -> Cow<str> {
        match &self.info {
            ClientConnectionInfo::UnixDomainSocket(sock) => sock.as_os_str().to_string_lossy(),
        }
    }

    pub fn send(&mut self, msg: FromEditor) -> anyhow::Result<()> {
        self.send.blocking_send(msg)?;
        Ok(())
    }
}

/// Information on how the client is connected
#[derive(Debug, Clone)]
pub(crate) enum ClientConnectionInfo {
    UnixDomainSocket(PathBuf),
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
                server_handle.send(ToEditor::Message(id, msg));
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
    let mut needs_flush = false;

    while let Some(msg) = server_recv.recv().await {
        match msg {
            FromEditor::Message(mut msg) => {
                // TODO move diffing away from here? This basically acts as a
                // middleware to diff changes before they are sent to the
                // client. Diffing is done here so editor can just send stuff
                // without diffing all the changes itself.

                match msg {
                    ClientMessage::Redraw(redraw) => match state.handle_redraw(redraw) {
                        Some(new_redraw) => {
                            msg = new_redraw.into();
                            needs_flush = true;
                        }
                        None => continue,
                    },
                    ClientMessage::Flush => {
                        if !needs_flush {
                            continue;
                        }

                        needs_flush = false;
                    }
                    _ => {}
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
