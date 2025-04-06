pub(crate) mod client;
mod grid;
pub(crate) mod input;
pub(crate) mod message;
pub(crate) mod terminal;
mod ui;

use std::{io, sync::mpsc, thread};

use sanedit_messages::{ClientMessage, Command, Message, Reader, Writer};

use crate::ui::{UIResult, UI};
pub use client::*;
use message::ClientInternalMessage;

// We have 2 tasks that need to be running
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(read: R, write: W, opts: SocketStartOptions)
where
    R: io::Read + Clone + Send + 'static,
    W: io::Write + 'static,
{
    let mut writer: Writer<_, Message> = Writer::new(write);
    let mut ui = UI::new().expect("Failed to start UI");
    let (tx, rx) = mpsc::channel();

    writer
        .write(Message::Hello(ui.window().size()))
        .expect("Failed to send hello");

    // Open file if exists
    if let Some(file) = opts.file.as_ref() {
        writer
            .write(Message::Command(Command::OpenFile(file.clone())))
            .expect("Failed to open file");
    }

    // Input thread
    let input_sender = tx.clone();
    thread::spawn(|| input::run_loop(input_sender));

    let read_sender = tx;
    thread::spawn(|| run_read_loop(read, read_sender));

    while let Ok(msg) = rx.recv() {
        use ClientInternalMessage::*;
        match msg {
            FromServer(msg) => match ui.handle_message(msg) {
                Ok(UIResult::Exit) => break,
                Ok(UIResult::Nothing) => {}
                Ok(UIResult::Resize) => {
                    let rect = ui.window();
                    let msg = Message::Resize(rect.size());

                    if let Err(_e) = writer.write(msg) {
                        log::error!("Client failed to send event to server");
                        break;
                    }
                }
                Err(e) => {
                    log::error!("UI failed to handle message: {e}");
                    break;
                }
            },
            ToServer(mut msg) => {
                ui.on_send_input(&msg);

                match msg {
                    Message::Resize(size) => {
                        if let Err(e) = ui.resize(size) {
                            log::error!("Failed to resize UI: {e}");
                            break;
                        }
                        let rect = ui.window();
                        msg = Message::Resize(rect.size());
                    }
                    Message::MouseEvent(ref mut ev) => {
                        let win = ui.window();
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
            Focus(focus) => ui.on_focus_change(focus),
        }
    }

    drop(ui);
    std::process::exit(0);
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
