// Events sent between client and editor

use tokio::io;

use crate::server::{ClientId, ClientHandle};

pub(crate) enum ToServer {
    NewClient(ClientHandle),
    Message(ClientId, Message),
    FatalError(io::Error),
}
pub(crate) enum FromServer {}

pub(crate) enum Message {
}
