// An event is a message which informs various listeners about something which
// has happened.
// Commands trigger something which should happen (in the future).

/// Messages sent to the client
pub enum ClientMessage {
    Hello,
    Redraw,
    Flush,
    Bye,
}

/// Messages sent to the server
pub enum Message {
    Hello,
    KeyEvent(),
    MouseEvent(),
    Resize,
    Bye,
}
