pub mod unix;

use std::{
    collections::VecDeque,
    io::{self, BufReader, Read},
    os::unix::net::UnixStream,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use sanedit_messages::{ClientMessage, Message, Reader};

use crate::input;

// We have 3 tasks that need to be running
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(mut read: R, mut write: W)
where
    R: io::Read,
    W: io::Write,
{
    // Other threads check this flag once in a while and stop if it is true.
    const STOP: AtomicBool = AtomicBool::new(false);

    // Input thread
    // IDEA: Send inputs to logic task and logic task sends them to server if needed?
    // let input_join = thread::spawn(|| input::run_loop(&STOP));

    // Read thread:
    // IDEA: is this needed? could you just read here => probably
    // let read_join = thread::spawn(|| conn_read(read, send, &STOP));

    write.write(&Message::Hello.serialize().unwrap()).unwrap();

    // -----------------
    let mut reader = Reader::new(read);
    loop {
        match ClientMessage::deserialize(reader.buffer()) {
            Ok(msg) => {
                let size = msg.serialized_size().expect("ser size");
                reader.consume(size);
                println!("MSG: {:?}", msg);
            }
            Err(e) => match e {
                sanedit_messages::Error::Io(_) => break,
                sanedit_messages::Error::InvalidData => reader.consume(1),
                sanedit_messages::Error::NeedMore => {
                    reader.more().expect("read more");
                }
            },
        }
    }

    STOP.store(true, Ordering::SeqCst);

    // read_join.join();
    // input_join.join();
}
