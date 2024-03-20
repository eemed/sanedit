// Events sent between client and editor

use sanedit_messages::{ClientMessage, Message};
use tokio::io;

use crate::server::{ClientHandle, ClientId, FromJobs};

#[derive(Debug)]
pub(crate) enum ToEditor {
    NewClient(ClientHandle),
    Jobs(FromJobs),
    Redraw,
    Message(ClientId, Message),
    FatalError(io::Error),
}

#[derive(Debug)]
pub(crate) enum FromEditor {
    Message(ClientMessage),
}

impl From<ClientMessage> for FromEditor {
    fn from(msg: ClientMessage) -> Self {
        FromEditor::Message(msg)
    }
}
