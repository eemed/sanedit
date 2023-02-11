mod buffers;
pub(crate) mod jobs;
mod keymap;
mod themes;
pub(crate) mod windows;

use sanedit_messages::redraw;
use sanedit_messages::redraw::Redraw;
use sanedit_messages::redraw::Size;
use sanedit_messages::redraw::Theme;
use sanedit_messages::ClientMessage;
use sanedit_messages::KeyEvent;
use sanedit_messages::Message;

use std::collections::HashMap;
use std::env;
use std::mem;
use std::path::Path;
use std::path::PathBuf;

use tokio::io;
use tokio::sync::mpsc::Receiver;

use crate::actions::prompt;
use crate::actions::text;
use crate::actions::Action;
use crate::editor::buffers::buffer::Buffer;
use crate::events::ToEditor;
use crate::server::ClientHandle;
use crate::server::ClientId;
use crate::server::JobsHandle;

use self::buffers::Buffers;
use self::jobs::Jobs;
use self::keymap::Keymap;
use self::windows::Mode;
use self::windows::Window;
use self::windows::Windows;

pub(crate) struct Editor {
    clients: HashMap<ClientId, ClientHandle>,
    windows: Windows,
    buffers: Buffers,
    jobs: Jobs,
    keymap: Keymap,
    prompt_keymap: Keymap,
    keys: Vec<KeyEvent>,
    is_running: bool,
    working_dir: PathBuf,
    themes: HashMap<String, Theme>,
}

impl Editor {
    fn new(jobs_handle: JobsHandle) -> Editor {
        Editor {
            clients: HashMap::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            jobs: Jobs::new(jobs_handle),
            keymap: Keymap::default_normal(),
            prompt_keymap: Keymap::default_prompt(),
            keys: Vec::default(),
            is_running: true,
            working_dir: env::current_dir().expect("Cannot get current working directory."),
            themes: themes::default_themes(),
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
        let client_ids: Vec<ClientId> = self.clients.iter().map(|(id, _)| *id).collect();
        for id in client_ids {
            log::info!("Quit to {:?}", id);
            // Dont care about errors here we are quitting anyway
            let _ = self.send_to_client(id, ClientMessage::Bye.into());
        }
        self.is_running = false;
    }

    fn on_client_connected(&mut self, handle: ClientHandle) {
        log::info!("Client connected: {:?}, id: {:?}", handle.info, handle.id);
        self.clients.insert(handle.id, handle);
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
            Message::Hello(size) => {
                let bid = self.buffers.insert(Buffer::default());
                self.windows.new_window(id, bid, size.width, size.height);

                let win = self.windows.get(id).expect("Window not present");
                let theme = {
                    let theme_name = &win.options.display.theme;
                    self.themes
                        .get(theme_name.as_str())
                        .expect("Theme not present")
                        .clone()
                };
                self.send_to_client(id, ClientMessage::Hello);
                self.send_to_client(id, ClientMessage::Theme(theme));
            }
            Message::KeyEvent(key_event) => self.handle_key_event(id, key_event),
            Message::MouseEvent(_) => {}
            Message::Resize(size) => self.handle_resize(id, size),
            Message::Bye => {}
        }

        self.redraw(id);
    }

    fn handle_resize(&mut self, id: ClientId, size: Size) {
        let win = self.windows.get_mut(id).expect("Client window is closed");
        win.resize(size);
    }

    fn redraw(&mut self, id: ClientId) {
        let win = self.windows.get_mut(id).expect("Client window is closed");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("Window referencing non existent buffer");

        let theme = {
            let theme_name = &win.options.display.theme;
            self.themes
                .get(theme_name.as_str())
                .expect("Theme not present")
        };
        let messages = win.redraw(buf, theme);

        for msg in messages {
            self.send_to_client(id, ClientMessage::Redraw(msg));
        }

        self.send_to_client(id, ClientMessage::Flush);
    }

    fn get_bound_action(&mut self, id: ClientId) -> Option<Action> {
        let (win, _buf) = self.get_win_buf(id);
        let keymap = match win.mode() {
            Mode::Normal => &self.keymap,
            Mode::Prompt => &self.prompt_keymap,
        };

        match keymap.get(&self.keys) {
            keymap::KeymapResult::Matched(action) => Some(action),
            _ => None,
        }
    }

    fn handle_key_event(&mut self, id: ClientId, event: KeyEvent) {
        use sanedit_messages::Key::*;

        // Add key to buffer
        self.keys.push(event);

        // Handle key bindings
        if let Some(mut action) = self.get_bound_action(id) {
            self.keys.clear();
            action.execute(self, id);
            return;
        }

        // Clear keys buffer, and handle them separately
        let events = mem::replace(&mut self.keys, vec![]);
        for event in events {
            match event.key() {
                Char(ch) => {
                    let mut buf = [0u8; 4];
                    let string = ch.encode_utf8(&mut buf);
                    self.handle_insert(id, string);
                }
                Tab => self.handle_insert(id, "\t"),
                Enter => {
                    let eol = {
                        let (_, buf) = self.get_win_buf(id);
                        buf.options().eol
                    };
                    self.handle_insert(id, eol.as_str());
                }
                _ => {}
            }
        }
    }

    fn handle_insert(&mut self, id: ClientId, text: &str) {
        let (win, _buf) = self.get_win_buf(id);

        // Where to send input
        match win.mode() {
            Mode::Normal => text::insert_at_cursor(self, id, text),
            Mode::Prompt => prompt::prompt_insert_at_cursor(self, id, text),
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn jobs_mut(&mut self) -> &mut Jobs {
        &mut self.jobs
    }

    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) fn main_loop(
    jobs_handle: JobsHandle,
    mut recv: Receiver<ToEditor>,
) -> Result<(), io::Error> {
    let mut editor = Editor::new(jobs_handle);

    while let Some(msg) = recv.blocking_recv() {
        match msg {
            ToEditor::NewClient(handle) => editor.on_client_connected(handle),
            ToEditor::Jobs(msg) => {
                log::info!("msg from jobs {msg:?}");
            }
            ToEditor::Message(id, msg) => editor.handle_message(id, msg),
            ToEditor::FatalError(e) => {
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
