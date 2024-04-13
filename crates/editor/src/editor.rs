pub(crate) mod buffers;
pub(crate) mod clipboard;
pub(crate) mod hooks;
pub(crate) mod job_broker;
pub(crate) mod keymap;
pub(crate) mod options;
pub(crate) mod syntax;
pub(crate) mod themes;
pub(crate) mod windows;

use rustc_hash::FxHashMap;
use sanedit_messages::redraw::Size;
use sanedit_messages::ClientMessage;
use sanedit_messages::KeyEvent;
use sanedit_messages::KeyMods;
use sanedit_messages::Message;
use sanedit_messages::MouseButton;
use sanedit_messages::MouseEvent;
use sanedit_messages::MouseEventKind;

use std::env;
use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::time::Instant;

use tokio::io;

use crate::actions;
use crate::actions::cursors;
use crate::actions::hooks::run;
use crate::common::dirs::ConfigDirectory;
use crate::common::file::File;
use crate::draw::DrawState;
use crate::editor::buffers::Buffer;
use crate::editor::hooks::Hook;
use crate::editor::keymap::KeymapResult;
use crate::events::ToEditor;
use crate::job_runner::spawn_job_runner;
use crate::job_runner::FromJobs;
use crate::runtime::TokioRuntime;
use crate::server::ClientHandle;
use crate::server::ClientId;
use crate::StartOptions;

use self::buffers::BufferId;
use self::buffers::Buffers;
use self::hooks::Hooks;
use self::job_broker::JobBroker;
use self::options::Options;

use self::syntax::Syntaxes;
use self::themes::Themes;
use self::windows::Window;
use self::windows::Windows;

pub(crate) struct Editor {
    clients: FxHashMap<ClientId, ClientHandle>,
    draw_states: FxHashMap<ClientId, DrawState>,
    windows: Windows,
    buffers: Buffers,
    keys: Vec<KeyEvent>,
    is_running: bool,

    pub runtime: TokioRuntime,
    pub themes: Themes,
    pub working_dir: PathBuf,
    pub config_dir: ConfigDirectory,
    pub syntaxes: Syntaxes,
    pub job_broker: JobBroker,
    pub hooks: Hooks,
    pub options: Options,
}

impl Editor {
    fn new(runtime: TokioRuntime) -> Editor {
        let handle = runtime.editor_handle();
        // Spawn job runner
        let jobs_handle = runtime.block_on(spawn_job_runner(handle));

        Editor {
            runtime,
            clients: FxHashMap::default(),
            draw_states: FxHashMap::default(),
            syntaxes: Syntaxes::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            job_broker: JobBroker::new(jobs_handle),
            hooks: Hooks::default(),
            keys: Vec::default(),
            is_running: true,
            config_dir: ConfigDirectory::default(),
            working_dir: env::current_dir().expect("Cannot get current working directory."),
            themes: Themes::default(),
            options: Options::default(),
        }
    }

    pub fn configure(&mut self, mut opts: StartOptions) {
        if let Some(cd) = opts.config_dir.take() {
            let cd = ConfigDirectory::new(&cd);
            self.config_dir = cd;
            self.syntaxes = Syntaxes::new(&self.config_dir.filetype_dir());
            self.themes = Themes::new(&self.config_dir.theme_dir());
        }
    }

    /// Ran after the startup configuration is complete
    pub fn on_startup(&mut self) {
        self.themes.load_all();
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

    pub fn windows(&self) -> &Windows {
        &self.windows
    }

    pub fn quit_client(&mut self, id: ClientId) {
        let _ = self.send_to_client(id, ClientMessage::Bye.into());

        if let Some(win) = self.windows.remove(id) {
            let old = win.buffer_id();
            let is_used = self.windows.iter().any(|(_, win)| win.buffer_id() == old);
            if !is_used {
                self.buffers.remove(old);
            }
        }

        self.draw_states.remove(&id);
        self.clients.remove(&id);
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

    /// Open a buffer in window
    fn open_buffer(&mut self, id: ClientId, bid: BufferId) {
        let (win, _) = self.win_buf_mut(id);
        // TODO check if buffer is not saved and what to do if it is not, prompt
        // save or auto save?

        let old = win.open_buffer(bid);
        // Remove if unused
        let is_used = self.windows.iter().any(|(_, win)| win.buffer_id() == old);
        if !is_used {
            self.buffers.remove(old);
        }

        run(self, id, Hook::BufOpened);
    }

    /// Open a file in window
    pub fn open_file(&mut self, id: ClientId, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref().canonicalize()?;

        // Use existing if possible
        let bid = match self.buffers.find(&path) {
            Some(bid) => bid,
            None => {
                let file = File::new(&path, &self.options)?;
                let buf = Buffer::from_file(file)?;
                self.buffers.insert(buf)
            }
        };
        self.open_buffer(id, bid);

        Ok(())
    }

    pub fn send_to_client(&mut self, id: ClientId, msg: ClientMessage) {
        if let Err(_e) = self.clients[&id].send.blocking_send(msg.into()) {
            log::info!(
                "Server failed to send to client {:?}, removing from client map",
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
            _ => {}
        }

        run(self, id, Hook::OnMessagePre);

        match msg {
            Message::KeyEvent(key_event) => self.handle_key_event(id, key_event),
            Message::MouseEvent(mouse_event) => self.handle_mouse_event(id, mouse_event),
            Message::Resize(size) => self.handle_resize(id, size),
            _ => {}
        }
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
                let (_win, _buf) = self.win_buf_mut(id);
                if event.mods.contains(KeyMods::CONTROL) {
                    cursors::new_to_point(self, id, event.point);
                } else if event.mods.is_empty() {
                    cursors::goto_position(self, id, event.point);
                }
            }
            _ => {}
        }
    }

    fn redraw_all(&mut self) {
        log::info!("Redraw all");
        let clients: Vec<ClientId> = self.clients.keys().cloned().collect();

        for cid in clients {
            self.redraw(cid);
        }
    }

    /// Redraw all windows that use the same buffer as `id`
    fn redraw(&mut self, id: ClientId) {
        // Editor is closed or client is closed
        if !self.is_running || !self.clients.contains_key(&id) {
            return;
        }

        if let Some(bid) = self.windows.bid(id) {
            for cid in self.windows.find_clients_with_buf(bid) {
                self.redraw_client(cid);
            }
        }
    }

    /// redraw a window
    fn redraw_client(&mut self, id: ClientId) {
        run(self, id, Hook::OnDrawPre);

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

    fn handle_key_event(&mut self, id: ClientId, event: KeyEvent) {
        log::info!("Editor got {event}");
        use sanedit_messages::Key::*;

        // Add key to buffer
        self.keys.push(event);
        run(self, id, Hook::KeyPressedPre);

        // Handle key bindings
        let (win, _buf) = self.win_buf(id);
        let keymap = win.focus_keymap();
        match keymap.get(&self.keys) {
            KeymapResult::Matched(action) => {
                self.keys.clear();
                action.execute(self, id);
                return;
            }
            KeymapResult::Pending => return,
            KeymapResult::NotFound => {}
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
                    insert(self, id, eol.as_ref());
                }
                _ => {}
            }
        }
    }

    pub fn handle_job_msg(&mut self, msg: FromJobs) {
        use FromJobs::*;
        match msg {
            Message(id, msg) => {
                if let Some(prog) = self.job_broker.get(id) {
                    prog.on_message(self, msg);
                }
            }
            Succesful(id) => {
                log::info!("Job {id} succesful.");
                if let Some(prog) = self.job_broker.get(id) {
                    prog.on_success(self);
                }
                self.job_broker.done(id);
            }
            Failed(id, reason) => {
                log::info!("Job {id} failed because {}.", reason);
                if let Some(prog) = self.job_broker.get(id) {
                    prog.on_failure(self, &reason);
                }
                self.job_broker.done(id);
            }
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    pub fn draw_state(&mut self, id: ClientId) -> &mut DrawState {
        self.draw_states.get_mut(&id).unwrap()
    }

    pub fn reload(&mut self, id: ClientId) {
        // Reload theme
        let (win, buf) = self.win_buf(id);
        let theme = win.view().options.theme.clone();
        if let Ok(theme) = self.themes.load(&theme).cloned() {
            self.send_to_client(id, ClientMessage::Theme(theme))
        }

        // Reload syntax
        let (win, buf) = self.win_buf(id);
        if let Some(ft) = buf.filetype.clone() {
            let _ = self.syntaxes.load(&ft);
        }

        // Reload window
        let (win, buf) = self.win_buf_mut(id);
        win.reload();

        run(self, id, Hook::Reload);
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) fn main_loop(
    runtime: TokioRuntime,
    recv: Receiver<ToEditor>,
    opts: StartOptions,
) -> Result<(), io::Error> {
    let mut editor = Editor::new(runtime);
    editor.configure(opts);
    editor.on_startup();

    let framerate = Duration::from_millis(1000 / 30);
    let mut redraw = Instant::now();
    let mut was_previously_redrawn = false;

    while editor.is_running {
        match recv.recv_timeout(framerate) {
            Ok(msg) => {
                was_previously_redrawn = false;

                use ToEditor::*;
                match msg {
                    NewClient(handle) => editor.on_client_connected(handle),
                    Jobs(msg) => editor.handle_job_msg(msg),
                    Message(id, msg) => editor.handle_message(id, msg),
                    FatalError(e) => {
                        log::info!("Fatal error: {}", e);
                        break;
                    }
                }

                let now = Instant::now();
                if now.duration_since(redraw) > framerate {
                    was_previously_redrawn = true;
                    redraw = Instant::now();
                    editor.redraw_all();
                }
            }
            Err(_) => {
                if !was_previously_redrawn {
                    was_previously_redrawn = true;
                    redraw = Instant::now();
                    editor.redraw_all();
                }
            }
        }
    }

    Ok(())
}
