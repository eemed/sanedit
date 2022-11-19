pub mod unix;

use std::{
    io::{self, Read},
    os::unix::net::UnixStream,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use sanedit_messages::ClientMessage;

use crate::input;

fn conn_read(recv: impl io::Read, send: Sender<ClientMessage>, stop: &AtomicBool) {}

// We have 3 tasks that need to be running
// Read thread : reads bytes from connection to message channel
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(read: R, write: W)
where
    R: io::Read,
    W: io::Write + Clone,
{
    // Other threads check this flag once in a while and stop if it is true.
    const STOP: AtomicBool = AtomicBool::new(false);

    // Channel for reader task to send us the messages
    let (send, recv): (Sender<ClientMessage>, Receiver<ClientMessage>) = mpsc::channel();

    // Input thread
    // IDEA: Send inputs to logic task and logic task sends them to server if needed?
    let input_join = thread::spawn(|| input::run_loop(&STOP));

    // Read thread:
    // IDEA: is this needed? could you just read here => probably
    let read_join = thread::spawn(|| conn_read(read, send, &STOP));

    // Logic thread
    while let Ok(msg) = recv.recv() {
        match msg {
            ClientMessage::Hello => {}
            ClientMessage::Redraw(_) => todo!(),
            ClientMessage::Flush => todo!(),
            ClientMessage::Bye => todo!(),
        }
    }

    STOP.store(true, Ordering::SeqCst);

    read_join.join();
    input_join.join();
}
