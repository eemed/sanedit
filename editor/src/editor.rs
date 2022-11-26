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

pub(crate) struct Editor {
    pub(crate) clients: HashMap<ClientId, ClientHandle>,
    pub(crate) windows: HashMap<ClientId, Window>,
    pub(crate) buffers: SlotMap<BufferId, Buffer>,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            clients: HashMap::new(),
            windows: HashMap::new(),
            buffers: SlotMap::with_key(),
        }
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) async fn main_loop(mut recv: Receiver<ToServer>) -> Result<(), io::Error> {
    println!("Main loop");
    let mut editor = Editor::new();

    while let Some(msg) = recv.recv().await {
        match msg {
            ToServer::NewClient(handle) => {
                println!("Client connected: {:?}, id: {:?}", handle.info, handle.id);
                editor.clients.insert(handle.id, handle);
            }
            ToServer::Message(id, msg) => handle_msg(&mut editor, id, msg).await?,
            ToServer::FatalError(e) => {
                println!("Fatal error: {}", e);
                break;
            }
        }
    }

    println!("Main loop exiting");
    Ok(())
}

async fn handle_msg(editor: &mut Editor, id: ClientId, msg: Message) -> Result<(), io::Error> {
    println!("Message {:?} from client {:?}", msg, id);
    match msg {
        Message::Hello => {
            if let Err(_e) = editor.clients[&id]
                .send
                .send(ClientMessage::Hello.into())
                .await
            {
                println!(
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
