pub(crate) mod client;
mod grid;
pub(crate) mod input;
pub(crate) mod message;
mod signals;
pub(crate) mod terminal;
mod ui;
mod window_manager;

use std::{io, thread};

use crossbeam::channel::{Select, Sender};
use sanedit_messages::{ClientMessage, Command, Message, Reader, Writer};
use window_manager::WindowManager;

use crate::ui::{UIResult, UI};
pub use client::*;
pub use message::ClientInternalMessage;

// We have 2 tasks that need to be running
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(read: R, write: W, mut opts: ClientOptions)
where
    R: io::Read + Clone + Send + 'static,
    W: io::Write + 'static,
{
    let mut writer: Writer<_, Message> = Writer::new(write);
    let mut ui = UI::new().expect("Failed to start UI");
    let (tx, rx) = crossbeam::channel::unbounded();
    let (internal_tx, internal_rx) = crossbeam::channel::unbounded();
    let color_count = ui.color_count();

    signals::register_signal_handlers(internal_tx.clone());

    writer
        .write(Message::Hello {
            color_count,
            size: ui.window().size(),
            parent: opts.parent_client,
        })
        .expect("Failed to send hello");

    // Open file if exists
    if let Some(file) = opts.file.take() {
        let command = match file {
            InitialFile::Path(path_buf) => Command::OpenFile {
                path: path_buf,
                language: opts.language,
            },
            InitialFile::Stdin(vec) => Command::ReadStdin {
                bytes: vec,
                language: opts.language,
            },
        };

        writer
            .write(Message::Command(command))
            .expect("Failed to send command");
    }

    // Input thread
    let itx = internal_tx.clone();
    thread::spawn(|| input::run_loop(itx));

    let read_sender = tx;
    thread::spawn(|| run_read_loop(read, read_sender));

    let mut recv_select = Select::new_biased();
    // Prioritize internal over outside
    let receivers = [internal_rx, rx];
    for recv in &receivers {
        recv_select.recv(recv);
    }

    let mut wm = None;

    while let Ok(msg) = {
        let oper = recv_select.select();
        let index = oper.index();
        oper.recv(&receivers[index])
    } {
        use ClientInternalMessage::*;
        match msg {
            FromServer(ClientMessage::Hello { id }) => {
                wm = Some(WindowManager::new(id, &opts.session));
            }
            FromServer(ClientMessage::SplitVertical) => {
                if let Some(ref mut wm) = wm {
                    wm.new_window_vertical();
                }
            }
            FromServer(ClientMessage::SplitHorizontal) => {
                if let Some(ref mut wm) = wm {
                    wm.new_window_horizontal();
                }
            }
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
                    Message::MouseEvent(ev) => match ui.handle_mouse_event(ev) {
                        Some(mmsg) => msg = mmsg,
                        _ => continue,
                    },
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
            Bye => {
                let _ = writer.write(Message::Bye);
                break;
            }
            Focus(focus) => {
                ui.on_focus_change(focus);
                let msg = if focus {
                    Message::FocusGained
                } else {
                    Message::FocusLost
                };
                if let Err(_e) = writer.write(msg) {
                    log::error!("Client failed to send event to server");
                    break;
                }
            }
        }
    }

    drop(ui);
}

fn run_read_loop<R>(read: R, sender: Sender<ClientInternalMessage>)
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
