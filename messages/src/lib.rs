// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

mod codec;
mod key;
mod mouse;
mod reader;
pub mod redraw;
mod writer;

pub use codec::BinCodec;
pub use key::{try_parse_keyevents, Key, KeyEvent, KeyMods};
pub use mouse::MouseEvent;
pub use reader::Reader;
use redraw::Redraw;
use serde::{Deserialize, Serialize};
pub use tokio_util::codec::{Decoder, Encoder};
pub use writer::{WriteError, Writer};

/// Messages sent to the client
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ClientMessage {
    Hello,
    Redraw(Redraw),
    Flush,
    Bye,
}

/// Messages sent to the server
#[derive(Serialize, Deserialize, PartialEq,Eq, Debug)]
pub enum Message {
    Hello,
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
    Resize,
    Bye,
}
