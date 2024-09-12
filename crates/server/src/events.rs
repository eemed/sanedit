// Events sent between client and editor

use sanedit_messages::{ClientMessage, Message};
use tokio::io;

use crate::{job_runner::FromJobs, ClientHandle, ClientId};

#[derive(Debug)]
pub enum ToEditor {
    NewClient(ClientHandle),
    Jobs(FromJobs),
    Message(ClientId, Message),
    FatalError(io::Error),
}

#[derive(Debug)]
pub enum FromEditor {
    Message(ClientMessage),
}

impl From<ClientMessage> for FromEditor {
    fn from(msg: ClientMessage) -> Self {
        FromEditor::Message(msg)
    }
}
