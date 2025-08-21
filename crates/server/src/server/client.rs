pub(crate) mod unix;

use std::{borrow::Cow, path::PathBuf, sync::Arc};

use futures_util::{SinkExt as _, StreamExt};
use sanedit_messages::{redraw::Redraw, BinCodec, ClientMessage, Message};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::events::{FromEditor, ToEditor};

use super::EditorHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClientId(pub(crate) usize);

impl ClientId {
    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn temporary(n: usize) -> ClientId {
        ClientId(n)
    }
}

impl From<ClientId> for String {
    fn from(value: ClientId) -> Self {
        value.0.to_string()
    }
}

#[derive(Debug)]
pub enum FromEditorSharedMessage {
    Shared {
        message: Arc<FromEditor>,
        sender: std::sync::mpsc::Sender<Arc<FromEditor>>,
    },
    Owned {
        message: FromEditor,
    },
}

impl From<ClientMessage> for FromEditorSharedMessage {
    fn from(message: ClientMessage) -> Self {
        FromEditorSharedMessage::Owned {
            message: message.into(),
        }
    }
}

impl From<Redraw> for FromEditorSharedMessage {
    fn from(value: Redraw) -> Self {
        FromEditorSharedMessage::Owned {
            message: FromEditor::Message(ClientMessage::Redraw(value)),
        }
    }
}

/// Client handle allows us to communicate with the client
#[derive(Debug)]
pub struct ClientHandle {
    pub(crate) id: ClientId,
    pub(crate) info: ClientConnectionInfo,
    pub(crate) send: Sender<FromEditorSharedMessage>,
    pub(crate) _kill: JoinHandle<()>,
}

impl ClientHandle {
    pub fn id(&self) -> ClientId {
        self.id
    }

    pub fn connection_info(&self) -> Cow<'_, str> {
        match &self.info {
            ClientConnectionInfo::UnixDomainSocket(sock) => sock.as_os_str().to_string_lossy(),
        }
    }

    pub fn send(&mut self, msg: FromEditorSharedMessage) -> anyhow::Result<()> {
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
    mut server_recv: Receiver<FromEditorSharedMessage>,
) -> Result<(), io::Error> {
    let codec: BinCodec<ClientMessage> = BinCodec::new();
    let mut writer = Box::pin(FramedWrite::new(write, codec));

    while let Some(msg) = server_recv.recv().await {
        match msg {
            FromEditorSharedMessage::Shared { message, sender } => {
                match message.as_ref() {
                    FromEditor::Message(client_message) => {
                        if let Err(e) = writer.send(client_message).await {
                            log::error!("conn_write error: {}", e);
                            break;
                        }
                    }
                }

                let _ = sender.send(message);
            }
            FromEditorSharedMessage::Owned { message } => match message {
                FromEditor::Message(client_message) => {
                    if let Err(e) = writer.send(client_message).await {
                        log::error!("conn_write error: {}", e);
                        break;
                    }
                }
            },
        };
    }

    Ok(())
}
