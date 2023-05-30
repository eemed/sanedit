pub(crate) mod buffers;
pub(crate) mod hooks;
pub(crate) mod jobs;
pub(crate) mod keymap;
pub(crate) mod options;
pub(crate) mod themes;
pub(crate) mod windows;

use sanedit_messages::redraw::Size;
use sanedit_messages::redraw::Theme;
use sanedit_messages::ClientMessage;
use sanedit_messages::KeyEvent;
use sanedit_messages::KeyMods;
use sanedit_messages::Message;
use sanedit_messages::MouseButton;
use sanedit_messages::MouseEvent;
use sanedit_messages::MouseEventKind;

use std::collections::HashMap;
use std::env;
use std::mem;
use std::path::Path;
use std::path::PathBuf;

use tokio::io;
use tokio::sync::mpsc::Receiver;

use crate::actions;
use crate::actions::cursors;
use crate::actions::Action;
use crate::common::file::File;
use crate::draw::DrawState;
use crate::editor::buffers::Buffer;
use crate::events::ToEditor;
use crate::server::ClientHandle;
use crate::server::ClientId;
use crate::server::FromJobs;
use crate::server::JobProgress;
use crate::server::JobsHandle;

use self::buffers::Buffers;
use self::hooks::Hooks;
use self::jobs::Jobs;
use self::options::Options;
use self::windows::Cursor;
use self::windows::Window;
use self::windows::Windows;

pub(crate) struct Editor {
    clients: HashMap<ClientId, ClientHandle>,
    draw_states: HashMap<ClientId, DrawState>,
    windows: Windows,
    buffers: Buffers,
    keys: Vec<KeyEvent>,
    is_running: bool,
    working_dir: PathBuf,
    themes: HashMap<String, Theme>,

    pub jobs: Jobs,
    pub hooks: Hooks,
    pub options: Options,
}

impl Editor {
    fn new(jobs_handle: JobsHandle) -> Editor {
        Editor {
            clients: HashMap::default(),
            draw_states: HashMap::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            jobs: Jobs::new(jobs_handle),
            hooks: Hooks::default(),
            keys: Vec::default(),
            is_running: true,
            working_dir: env::current_dir().expect("Cannot get current working directory."),
            themes: themes::default_themes(),
            options: Options::default(),
        }
    }

    pub fn win_buf(&self, id: ClientId) -> (&Window, &Buffer) {
        let win = self.windows.get(id).expect("no win for cliend id {id}");
        let bid = win.buffer_id();
        let buf = self
            .buffers
            .get(bid)
            .expect("no buffer for buffer id {bid}");
        (win, buf)
    }

    pub fn win_buf_mut(&mut self, id: ClientId) -> (&mut Window, &mut Buffer) {
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

    /// Put buffer to buffers list and open it in window
    pub fn open_buffer(&mut self, id: ClientId, buf: Buffer) {
        let bid = self.buffers.insert(buf);
        let (win, _) = self.win_buf_mut(id);
        // TODO check if buffer is not saved and what to do if it is not, prompt
        // save or auto save?

        let old = win.open_buffer(bid);
        // Remove if unused
        let is_used = self.windows.iter().any(|(_, win)| win.buffer_id() == old);
        if !is_used {
            self.buffers.remove(old);
        }
    }

    /// Open a file in window
    pub fn open_file(&mut self, id: ClientId, path: impl AsRef<Path>) -> io::Result<()> {
        let file = File::new(path, &self.options)?;
        let buf = Buffer::from_file(file)?;
        self.open_buffer(id, buf);
        Ok(())
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
        match msg {
            Message::Hello(size) => {
                self.handle_hello(id, size);
                return;
            }
            Message::KeyEvent(key_event) => self.handle_key_event(id, key_event),
            Message::MouseEvent(mouse_event) => self.handle_mouse_event(id, mouse_event),
            Message::Resize(size) => self.handle_resize(id, size),
            Message::Bye => {
                self.quit();
                return;
            }
        }

        self.redraw(id);
    }

    fn handle_hello(&mut self, id: ClientId, size: Size) {
        // Create buffer and window
        let bid = self.buffers.insert(Buffer::default());
        let win = self.windows.new_window(id, bid, size.width, size.height);
        let buf = self.buffers.get(bid).expect("Buffer not present");
        let theme = {
            let theme_name = &win.display_options().theme;
            self.themes
                .get(theme_name.as_str())
                .expect("Theme not present")
                .clone()
        };

        // Create draw state and send out initial draw
        let (draw, messages) = DrawState::new(win, buf, &theme);
        self.draw_states.insert(id, draw);

        self.send_to_client(id, ClientMessage::Hello);
        self.send_to_client(id, ClientMessage::Theme(theme));
        for msg in messages {
            self.send_to_client(id, ClientMessage::Redraw(msg));
        }
        self.send_to_client(id, ClientMessage::Flush);
    }

    fn handle_resize(&mut self, id: ClientId, size: Size) {
        let (win, buf) = self.win_buf_mut(id);
        win.resize(size, buf);
    }

    fn handle_mouse_event(&mut self, id: ClientId, event: MouseEvent) {
        // TODO keybindings
        match event.kind {
            MouseEventKind::ScrollDown => {
                let (win, buf) = self.win_buf_mut(id);
                win.scroll_down_n(buf, 3);
            }
            MouseEventKind::ScrollUp => {
                let (win, buf) = self.win_buf_mut(id);
                win.scroll_up_n(buf, 3);
            }
            MouseEventKind::ButtonDown(MouseButton::Left) => {
                let (win, buf) = self.win_buf_mut(id);
                if event.mods.contains(KeyMods::CONTROL) {
                    cursors::new_cursor_to_point(self, id, event.point);
                } else if event.mods.is_empty() {
                    cursors::cursor_goto_position(self, id, event.point);
                }
            }
            _ => {}
        }
    }

    fn redraw(&mut self, id: ClientId) {
        let draw = self
            .draw_states
            .get_mut(&id)
            .expect("Client window is closed");
        let win = self.windows.get_mut(id).expect("Client window is closed");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("Window referencing non existent buffer");

        let theme = {
            let theme_name = &win.display_options().theme;
            self.themes
                .get(theme_name.as_str())
                .expect("Theme not present")
        };

        let messages = draw.redraw(win, buf, theme);
        if !messages.is_empty() {
            for msg in messages {
                self.send_to_client(id, ClientMessage::Redraw(msg));
            }
            self.send_to_client(id, ClientMessage::Flush);
        }
    }

    fn get_bound_action(&mut self, id: ClientId) -> Option<Action> {
        let (win, _buf) = self.win_buf(id);
        let keymap = win.keymap();

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
            if event.alt_pressed() || event.control_pressed() {
                continue;
            }

            use actions::text::insert;
            match event.key() {
                Char(ch) => {
                    let mut buf = [0u8; 4];
                    let string = ch.encode_utf8(&mut buf);
                    insert(self, id, string);
                }
                Tab => insert(self, id, "\t"),
                Enter => {
                    let eol = {
                        let (_, buf) = self.win_buf(id);
                        buf.options().eol
                    };
                    insert(self, id, eol.as_str());
                }
                _ => {}
            }
        }
    }

    pub fn handle_job_msg(&mut self, msg: FromJobs) {
        match msg {
            FromJobs::Progress(id, progress) => match progress {
                JobProgress::Output(out) => {
                    if let Some((client_id, on_output)) = self.jobs.on_output_handler(&id) {
                        (on_output)(self, client_id, out);
                        self.redraw(client_id);
                    }
                }
                JobProgress::Error(out) => {
                    if let Some((client_id, on_error)) = self.jobs.on_error_handler(&id) {
                        (on_error)(self, client_id, out);
                        self.redraw(client_id);
                    }
                }
            },
            FromJobs::Ok(id) => {
                log::info!("Job {id} succesful.");
                self.jobs.done(&id);
            }
            FromJobs::Fail(id) => {
                log::info!("Job {id} failed.");
                self.jobs.done(&id);
            }
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
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
                editor.handle_job_msg(msg);
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
