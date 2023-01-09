mod bindings;
mod buffer;
mod window;

pub(crate) use buffer::*;
use sanedit_messages::ClientMessage;
use sanedit_messages::KeyEvent;
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
    clients: HashMap<ClientId, ClientHandle>,
    windows: HashMap<ClientId, Window>,
    buffers: SlotMap<BufferId, Buffer>,
    binds: KeyBindings,
    is_running: bool,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            clients: HashMap::new(),
            windows: HashMap::new(),
            buffers: SlotMap::with_key(),
            binds: KeyBindings::default(),
            is_running: true,
        }
    }

    fn quit(&mut self) {
        for (_, client) in &self.clients {
            // Dont care about errors here we are quitting anyway
            let _ = client.send.blocking_send(ClientMessage::Bye.into());
        }
        self.is_running = false;
    }

    fn on_client_connected(&mut self, handle: ClientHandle) {
        log::info!("Client connected: {:?}, id: {:?}", handle.info, handle.id);
        self.clients.insert(handle.id, handle);
    }

    fn send_to_client(&mut self, id: ClientId, msg: ClientMessage) {
        if let Err(_e) = self.clients[&id]
            .send
            .blocking_send(ClientMessage::Hello.into())
        {
            log::info!(
                "Server failed to send reponse for client {:?}, removing from client map",
                id
            );
            self.clients.remove(&id);
        }
    }

    fn handle_message(&mut self, id: ClientId, msg: Message) {
        log::info!("Message {:?} from client {:?}", msg, id);
        match msg {
            Message::Hello => self.send_to_client(id, ClientMessage::Hello),
            Message::KeyEvent(key_event) => self.handle_key_event(id, key_event),
            Message::MouseEvent(_) => {}
            Message::Resize => {}
            Message::Bye => {}
        }
    }

    fn handle_key_event(&mut self, id: ClientId, event: KeyEvent) {
        use sanedit_messages::Key::*;

        // Handle quit
        if *event.key() == Char('c') && event.control_pressed() {
            self.quit();
            return;
        }

        match event.key() {
            // Char(ch) => todo!()
            // F(n) => todo!(),
            // Enter => todo!(),
            // Esc => todo!(),
            // Tab => todo!(),
            // BackTab => todo!(),
            // Up => todo!(),
            // Down => todo!(),
            // Left => todo!(),
            // Right => todo!(),
            // Backspace => todo!(),
            // Delete => todo!(),
            // Home => todo!(),
            // End => todo!(),
            // PageUp => todo!(),
            // PageDown => todo!(),
            // Insert => todo!(),
            // Unknown => todo!(),
            _ => {}
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) fn main_loop(mut recv: Receiver<ToServer>) -> Result<(), io::Error> {
    let mut editor = Editor::new();

    while let Some(msg) = recv.blocking_recv() {
        match msg {
            ToServer::NewClient(handle) => editor.on_client_connected(handle),
            ToServer::Message(id, msg) => editor.handle_message(id, msg),
            ToServer::FatalError(e) => {
                log::info!("Fatal error: {}", e);
                break;
            }
        }

        if !editor.is_running() {
            break;
        }
    }

    Ok(())
}
