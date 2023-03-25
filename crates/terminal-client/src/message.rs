use sanedit_messages::{ClientMessage, Message};

#[derive(Debug)]
pub(crate) enum ClientInternalMessage {
    FromServer(ClientMessage),
    ToServer(Message),
    Error(String),
    Bye,
}

impl From<Message> for ClientInternalMessage {
    fn from(m: Message) -> Self {
        ClientInternalMessage::ToServer(m)
    }
}
