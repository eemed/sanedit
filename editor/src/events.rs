// Events sent between client and editor

use sanedit_messages::{ClientMessage, Message};
use tokio::io;

use crate::server::{ClientHandle, ClientId};

#[derive(Debug)]
pub(crate) enum ToServer {
    NewClient(ClientHandle),
    Message(ClientId, Message),
    FatalError(io::Error),
}

#[derive(Debug)]
pub(crate) enum FromServer {
    Message(ClientMessage),
}

impl From<ClientMessage> for FromServer {
    fn from(msg: ClientMessage) -> Self {
        FromServer::Message(msg)
    }
}
