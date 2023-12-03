pub(crate) mod client;
mod rectangle;
// mod grid;
pub(crate) mod input;
pub(crate) mod message;
pub(crate) mod terminal;
mod ui;

use std::{io, sync::mpsc, thread};

use sanedit_messages::{ClientMessage, Message, Reader, Writer};

use crate::ui::UI;
pub use client::*;
use message::ClientInternalMessage;

// We have 2 tasks that need to be running
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(read: R, write: W)
where
    R: io::Read + Clone + Send + 'static,
    W: io::Write + 'static,
{
    let mut writer: Writer<_, Message> = Writer::new(write);
    let mut ui = UI::new().expect("Failed to start UI");
    let (tx, rx) = mpsc::channel();

    writer
        .write(Message::Hello(ui.window_rect().size()))
        .expect("Failed to send hello");

    // Input thread
    let input_sender = tx.clone();
    let _input_join = thread::spawn(|| input::run_loop(input_sender));

    let read_sender = tx;
    let _read_join = thread::spawn(|| run_read_loop(read, read_sender));

    while let Ok(msg) = rx.recv() {
        use ClientInternalMessage::*;
        match msg {
            FromServer(msg) => {
                if ui.handle_message(msg) {
                    break;
                }
            }
            ToServer(mut msg) => {
                ui.on_send_input(&msg);

                match msg {
                    Message::Resize(size) => {
                        ui.resize(size);
                        let rect = ui.window_rect();
                        msg = Message::Resize(rect.size());
                    }
                    Message::MouseEvent(ref mut ev) => {
                        let win = ui.window_rect();
                        let position = win.position();
                        let size = win.size();
                        let point = &mut ev.point;

                        if point.x < position.x
                            || point.x >= position.x + size.width
                            || point.y < position.y
                            || point.y >= position.y + size.height
                        {
                            continue;
                        }
                        ev.point = ev.point - position;
                    }
                    _ => {}
                }

                if let Err(_e) = writer.write(msg) {
                    log::error!("Client failed to send event to server");
                    break;
                }
            }
            Error(e) => {
                log::error!("Client got error {}. Exiting.", e);
                break;
            }
            Bye => break,
        }
    }

    // Currently no way to message these that we are exiting
    // input_join.join().expect("Failed to join input thread");
    // read_join.join().expect("Failed to join read thread");
}

fn run_read_loop<R>(read: R, sender: mpsc::Sender<ClientInternalMessage>)
where
    R: io::Read + Clone + Send + 'static,
{
    let reader: Reader<_, ClientMessage> = Reader::new(read);

    for msg in reader {
        if let Err(_e) = sender.send(ClientInternalMessage::FromServer(msg)) {
            break;
        }
    }

    let _ = sender.send(ClientInternalMessage::Bye);
}
