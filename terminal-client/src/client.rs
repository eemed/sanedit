pub mod unix;

use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use sanedit_messages::{redraw::Redraw, ClientMessage, Message, Reader, Writer};

use crate::{input, terminal::Terminal, ui::UI};

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
    let mut ui = UI::new(writer).expect("Failed to start UI");
    let mut reader: Reader<_, ClientMessage> = Reader::new(read);

    for msg in reader {
        if ui.handle_message(msg) {
            break;
        }
    }
}
