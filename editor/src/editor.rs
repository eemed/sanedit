mod bindings;
mod buffer;
mod window;

pub(crate) use buffer::*;
use sanedit_messages::ClientMessage;
use sanedit_messages::Message;
use slotmap::SlotMap;
pub(crate) use window::*;

use std::collections::HashMap;

use tokio::io;
use tokio::sync::mpsc::Receiver;

use crate::events::ToServer;
use crate::server::ClientHandle;
use crate::server::ClientId;

use self::bindings::KeyBindings;

pub(crate) struct Editor {
    pub(crate) clients: HashMap<ClientId, ClientHandle>,
    pub(crate) windows: HashMap<ClientId, Window>,
    pub(crate) buffers: SlotMap<BufferId, Buffer>,
    pub(crate) binds: KeyBindings,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            clients: HashMap::new(),
            windows: HashMap::new(),
            buffers: SlotMap::with_key(),
            binds: KeyBindings::default(),
        }
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) fn main_loop(mut recv: Receiver<ToServer>) -> Result<(), io::Error> {
    log::info!("Main loop stating");
    let mut editor = Editor::new();

    while let Some(msg) = recv.blocking_recv() {
        match msg {
            ToServer::NewClient(handle) => {
                log::info!("Client connected: {:?}, id: {:?}", handle.info, handle.id);
                editor.clients.insert(handle.id, handle);
            }
            ToServer::Message(id, msg) => handle_msg(&mut editor, id, msg)?,
            ToServer::FatalError(e) => {
                log::info!("Fatal error: {}", e);
                break;
            }
        }
    }

    log::info!("Main loop exiting");
    Ok(())
}

fn handle_msg(editor: &mut Editor, id: ClientId, msg: Message) -> Result<(), io::Error> {
    log::info!("Message {:?} from client {:?}", msg, id);
    match msg {
        Message::Hello => {
            if let Err(_e) = editor.clients[&id]
                .send
                .blocking_send(ClientMessage::Hello.into())
            {
                log::info!(
                    "Server failed to send reponse for client {:?}, removing from client map",
                    id
                );
                editor.clients.remove(&id);
            }
        }
        Message::KeyEvent(_) => todo!(),
        Message::MouseEvent(_) => todo!(),
        Message::Resize => todo!(),
        Message::Bye => todo!(),
    }

    Ok(())
}
