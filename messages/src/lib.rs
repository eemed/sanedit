// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

mod codec;
mod key;
mod mouse;
mod reader;
mod redraw;
mod writer;

pub use codec::BinCodec;
pub use key::{Key, KeyEvent, KeyMods};
pub use mouse::MouseEvent;
pub use reader::Reader;
pub use redraw::Redraw;
use serde::{Deserialize, Serialize};
pub use tokio_util::codec::{Decoder, Encoder};
pub use writer::{WriteError, Writer};

/// Messages sent to the client
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ClientMessage {
    Hello,
    Redraw(Redraw),
    Flush,
    Bye,
}

/// Messages sent to the server
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
    Hello,
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
    Resize,
    Bye,
}
