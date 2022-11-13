// Events sent between client and editor

use tokio::io;

use crate::server::{ClientId, ClientHandle};

#[derive(Debug)]
pub(crate) enum ToServer {
    NewClient(ClientHandle),
    Message(ClientId, Message),
    FatalError(io::Error),
}

#[derive(Debug)]
pub(crate) enum FromServer {}

#[derive(Debug)]
pub(crate) enum Message {
}
