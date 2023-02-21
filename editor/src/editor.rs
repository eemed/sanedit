pub(crate) mod buffers;
pub(crate) mod jobs;
mod keymap;
pub(crate) mod options;
mod themes;
pub(crate) mod windows;

use sanedit_messages::redraw::Size;
use sanedit_messages::redraw::Theme;
use sanedit_messages::ClientMessage;
use sanedit_messages::KeyEvent;
use sanedit_messages::Message;
use sanedit_messages::MouseEvent;

use std::collections::HashMap;
use std::env;
use std::mem;
use std::path::Path;
use std::path::PathBuf;

use tokio::io;
use tokio::sync::mpsc::Receiver;

use crate::actions::prompt;
use crate::actions::prompt::prompt_file_conversion;
use crate::actions::text;
use crate::actions::Action;
use crate::common::file::FileMetadata;
use crate::draw::DrawState;
use crate::editor::buffers::Buffer;
use crate::events::ToEditor;
use crate::server::ClientHandle;
use crate::server::ClientId;
use crate::server::FromJobs;
use crate::server::JobProgress;
use crate::server::JobsHandle;

use self::buffers::Buffers;
use self::jobs::Jobs;
use self::keymap::Keymap;
use self::options::Convert;
use self::options::EditorOptions;
use self::windows::Mode;
use self::windows::Window;
use self::windows::Windows;

pub(crate) struct Editor {
    clients: HashMap<ClientId, ClientHandle>,
    draw_states: HashMap<ClientId, DrawState>,
    windows: Windows,
    buffers: Buffers,
    jobs: Jobs,
    keymap: Keymap,
    prompt_keymap: Keymap,
    keys: Vec<KeyEvent>,
    is_running: bool,
    working_dir: PathBuf,
    themes: HashMap<String, Theme>,
    pub options: EditorOptions,
}

impl Editor {
    fn new(jobs_handle: JobsHandle) -> Editor {
        Editor {
            clients: HashMap::default(),
            draw_states: HashMap::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            jobs: Jobs::new(jobs_handle),
            keymap: Keymap::default_normal(),
            prompt_keymap: Keymap::default_prompt(),
            keys: Vec::default(),
            is_running: true,
            working_dir: env::current_dir().expect("Cannot get current working directory."),
            themes: themes::default_themes(),
            options: EditorOptions::default(),
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

    /// Put buffer to buffers list and open it in client 'id':s window
    pub fn open_buffer(&mut self, id: ClientId, buf: Buffer) {
        let bid = self.buffers.insert(buf);
        let (win, _) = self.get_win_buf_mut(id);
        // TODO what to do with old buffer
        win.open_buffer(bid);
    }

    /// Open a file in client 'id':s window
    pub fn open_file(&mut self, id: ClientId, path: impl AsRef<Path>) -> io::Result<()> {
        let file = FileMetadata::new(path, &self.options)?;
        if !file.is_utf8() {
            match file.convert {
                Convert::Always => todo!(),
                Convert::Ask => {
                    prompt_file_conversion(self, id, file);
                    return Ok(());
                },
                Convert::Never => todo!(),
            }
        }

        let buf = Buffer::from_utf8_file(file)?;
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
        log::info!("Message {:?} from client {:?}", msg, id);
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
        let (win, buf) = self.get_win_buf_mut(id);
        win.resize(size, buf);
    }

    fn handle_mouse_event(&mut self, id: ClientId, event: MouseEvent) {
        // TODO keybindings
        match event {
            MouseEvent::ScrollDown => {
                let (win, buf) = self.get_win_buf_mut(id);
                win.scroll_down_n(buf, 3);
            }
            MouseEvent::ScrollUp => {
                let (win, buf) = self.get_win_buf_mut(id);
                win.scroll_up_n(buf, 3);
            }
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
            if event.alt_pressed() || event.control_pressed() {
                continue;
            }

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
                self.jobs.job_done(&id);
            }
            FromJobs::Fail(id) => {
                log::info!("Job {id} failed.");
                self.jobs.job_done(&id);
            }
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
