pub mod unix;

use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use sanedit_messages::{ClientMessage, Message, Reader, Writer};

use crate::{input, terminal::Terminal};

// We have 2 tasks that need to be running
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(read: R, mut write: W)
where
    R: io::Read + Clone + Send + 'static,
    W: io::Write + Clone + Send + 'static,
{
    // Other threads check this flag once in a while and stop if it is true.
    let stop = Arc::new(AtomicBool::new(false));
    let mut writer: Writer<_, Message> = Writer::new(write.clone());
    writer.write(Message::Hello).expect("Failed to send hello");

    // Input thread
    // IDEA: Send inputs to logic task and logic task sends them to server if needed?
    let cloned_write = write.clone();
    let cloned_stop = stop.clone();
    let input_join =
        thread::spawn(|| input::run_loop(cloned_write, cloned_stop).expect("Input loop failed"));

    run_logic_loop(read, write);
    stop.store(true, Ordering::Relaxed);
    input_join.join().expect("Failed to join input thread");
}

fn run_logic_loop<R, W>(read: R, mut write: W)
where
    R: io::Read + Clone + Send + 'static,
    W: io::Write + Clone + Send + 'static,
{
    let mut writer: Writer<_, Message> = Writer::new(write);
    let mut reader: Reader<_, ClientMessage> = Reader::new(read);
    let mut terminal = Terminal::new().expect("Failed to create terminal");

    for msg in reader {
        log::info!("Client got message: {:?}", msg);
        if handle_message(msg, &mut writer) {
            break;
        }
    }
}

fn handle_message<W: io::Write>(msg: ClientMessage, writer: &mut Writer<W, Message>) -> bool {
    match msg {
        ClientMessage::Hello => {}
        ClientMessage::Redraw(_) => {}
        ClientMessage::Flush => {}
        ClientMessage::Bye => return true,
    }

    return false;
}
