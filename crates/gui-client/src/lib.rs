pub mod client;
pub(crate) mod signals;
pub(crate) mod ui;

use std::io;

use eframe::egui;
use sanedit_messages::{redraw::Size, ClientMessage, Command, Message, Reader, Writer};

pub use client::*;

// We have 2 tasks that need to be running
// Input thread: polls inputs and writes them to the server.
// Logic thread: Reacts to server messages, draws screen.
pub fn run<R, W>(read: R, write: W, mut opts: ClientOptions)
where
    R: io::Read + Clone + Send + 'static,
    W: io::Write + Send + 'static,
{
    let mut writer: Writer<_, Message> = Writer::new(write);
    // let (internal_tx, internal_rx) = crossbeam::channel::unbounded();
    // let (tx, rx) = crossbeam::channel::unbounded();
    // signals::register_signal_handlers(internal_tx.clone());

        // writer
        //     .write(Message::Hello {
        //         color_count: 16_777_216,
        //         size: Size {
        //             width: 400,
        //             height: 400,
        //         },
        //         parent: opts.parent_client,
        //     })
        //     .expect("Failed to send hello");

        // // Open file if exists
        // if let Some(file) = opts.file.take() {
        //     let command = match file {
        //         InitialFile::Path(path_buf) => Command::OpenFile {
        //             path: path_buf,
        //             language: opts.language,
        //         },
        //         InitialFile::Stdin(vec) => Command::ReadStdin {
        //             bytes: vec,
        //             language: opts.language,
        //         },
        //     };

        //     writer
        //         .write(Message::Command(command))
        //         .expect("Failed to send command");
        // }

    let (context_send, context_recv) = crossbeam::channel::bounded::<egui::Context>(1);
    let (message_send, message_recv) = crossbeam::channel::unbounded::<Vec<ClientMessage>>();

    std::thread::spawn(move || {
        let Ok(ctx) = context_recv.recv() else {
            return;
        };

        let reader: Reader<_, ClientMessage> = Reader::new(read);
        let mut events = vec![];
        for msg in reader {
            match msg {
                ClientMessage::ConnectionTest => {}
                ClientMessage::Flush => {
                    message_send.send(std::mem::take(&mut events));
                    ctx.request_repaint();
                }
                e => events.push(e),
            }
        }
    });

    ui::run(context_send, message_recv, writer);
}
