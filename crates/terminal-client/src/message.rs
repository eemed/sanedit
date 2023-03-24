use sanedit_messages::{Message, ClientMessage};

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
