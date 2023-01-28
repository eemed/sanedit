mod buffers;
mod keymap;
mod windows;

use sanedit_messages::redraw;
use sanedit_messages::redraw::Redraw;
use sanedit_messages::ClientMessage;
use sanedit_messages::KeyEvent;
use sanedit_messages::Message;

use std::collections::HashMap;
use std::mem;

use tokio::io;
use tokio::sync::mpsc::Receiver;

use crate::actions::Action;
use crate::editor::buffers::buffer::Buffer;
use crate::events::ToServer;
use crate::server::ClientHandle;
use crate::server::ClientId;

use self::buffers::Buffers;
use self::keymap::Keymap;
use self::windows::window::Window;
use self::windows::Windows;

pub(crate) struct Editor {
    clients: HashMap<ClientId, ClientHandle>,
    windows: Windows,
    buffers: Buffers,
    keymap: Keymap,
    keys: Vec<KeyEvent>,
    is_running: bool,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            clients: HashMap::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            keymap: Keymap::default(),
            keys: Vec::default(),
            is_running: true,
        }
    }

    pub fn get_win_buf(&self, id: ClientId) -> (&Window, &Buffer) {
        let win = self.windows.get(id).expect("no win for cliend id {id}");
        let bid = win.buffer_id();
        let buf = self
            .buffers
            .get(bid)
            .expect("no buffer for buffer id {bid}");
        (win, buf)
    }

    pub fn get_win_buf_mut(&mut self, id: ClientId) -> (&mut Window, &mut Buffer) {
        let win = self.windows.get_mut(id).expect("no win for cliend id {id}");
        let bid = win.buffer_id();
        let buf = self
            .buffers
            .get_mut(bid)
            .expect("no buffer for buffer id {bid}");
        (win, buf)
    }

    pub fn quit(&mut self) {
        log::info!("Quit");
        for (_, client) in &self.clients {
            log::info!("Quit to {:?}", client.id);
            // Dont care about errors here we are quitting anyway
            let _ = client.send.blocking_send(ClientMessage::Bye.into());
        }
        self.is_running = false;
    }

    fn on_client_connected(&mut self, handle: ClientHandle) {
        log::info!("Client connected: {:?}, id: {:?}", handle.info, handle.id);
        let id = handle.id;
        self.clients.insert(handle.id, handle);
        let bid = self.buffers.insert(Buffer::new());
        self.windows.new_window(id, bid, 80, 20);
    }

    fn send_to_client(&mut self, id: ClientId, msg: ClientMessage) {
        if let Err(_e) = self.clients[&id].send.blocking_send(msg.into()) {
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

        self.redraw(id);
    }

    fn redraw(&mut self, id: ClientId) {
        let win = self.windows.get_mut(id).expect("Client window is closed");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("Window referencing non existent buffer");
        win.redraw(buf);
        let win: redraw::Window = win.view().into();
        let msg = ClientMessage::Redraw(Redraw::Window(win));
        self.send_to_client(id, msg);
        self.send_to_client(id, ClientMessage::Flush);
    }

    fn get_bound_action(&mut self) -> Option<Action> {
        match self.keymap.get(&self.keys) {
            keymap::KeymapResult::Matched(action) => {
                self.keys.clear();
                Some(action)
            }
            _ => None,
        }
    }

    fn handle_key_event(&mut self, id: ClientId, event: KeyEvent) {
        use sanedit_messages::Key::*;

        // Add key to buffer
        self.keys.push(event);

        // Handle key bindings
        if let Some(mut action) = self.get_bound_action() {
            action.execute(self, id);
            return;
        }

        let mut win = self.windows.get_mut(id).expect("Client window is closed");
        let mut buf = self
            .buffers
            .get_mut(win.buffer_id())
            .expect("Window referencing non existent buffer");

        // Clear keys buffer, and handle them separately
        let events = mem::replace(&mut self.keys, vec![]);
        for event in events {
            match event.key() {
                Char(ch) => buf.insert_char(0, *ch),
                // Enter => todo!(),
                // Tab => todo!(),
                // Backspace => todo!(),
                // Delete => todo!(),
                // F(n) => todo!(),
                // Esc => todo!(),
                // BackTab => todo!(),
                // Up => todo!(),
                // Down => todo!(),
                // Left => todo!(),
                // Right => todo!(),
                // Home => todo!(),
                // End => todo!(),
                // PageUp => todo!(),
                // PageDown => todo!(),
                // Insert => todo!(),
                // Unknown => todo!(),
                _ => {}
            }
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
