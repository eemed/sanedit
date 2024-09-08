pub(crate) mod buffers;
pub(crate) mod clipboard;
pub(crate) mod config;
pub(crate) mod filetree;
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

use tokio::io;

use anyhow::Result;

use crate::actions;
use crate::actions::cursors;
use crate::actions::hooks::run;
use crate::actions::jobs::LSPHandle;
use crate::common::dirs::ConfigDirectory;
use crate::common::file::FileDescription;
use crate::common::text::copy_cursors_to_lines;
use crate::common::text::paste_separate_cursor_lines;
use crate::draw::DrawState;
use crate::draw::EditorContext;
use crate::editor::buffers::Buffer;
use crate::editor::config::Config;
use crate::editor::hooks::Hook;
use crate::editor::keymap::KeymapResult;
use crate::editor::windows::Focus;
use crate::events::ToEditor;
use crate::job_runner::spawn_job_runner;
use crate::job_runner::FromJobs;
use crate::runtime::TokioRuntime;
use crate::server::ClientHandle;
use crate::server::ClientId;
use crate::StartOptions;

use self::buffers::BufferId;
use self::buffers::Buffers;
use self::buffers::Filetype;
use self::clipboard::Clipboard;
use self::clipboard::DefaultClipboard;
use self::hooks::Hooks;
use self::job_broker::JobBroker;
use self::keymap::DefaultKeyMappings;
use self::keymap::Keymap;
use self::keymap::KeymapKind;
use self::options::Options;

use self::filetree::Filetree;
use self::syntax::Syntaxes;
use self::themes::Themes;
use self::windows::History;
use self::windows::HistoryKind;
use self::windows::Window;
use self::windows::Windows;

/// Type to use with all hashmaps
pub(crate) type Map<K, V> = FxHashMap<K, V>;

pub(crate) struct Editor {
    clients: Map<ClientId, ClientHandle>,
    draw_states: Map<ClientId, DrawState>,
    windows: Windows,
    buffers: Buffers,
    keys: Vec<KeyEvent>,
    is_running: bool,
    working_dir: PathBuf,

    pub runtime: TokioRuntime,
    pub themes: Themes,
    pub config_dir: ConfigDirectory,
    pub syntaxes: Syntaxes,
    pub job_broker: JobBroker,
    pub hooks: Hooks,
    pub options: Options,
    pub clipboard: Box<dyn Clipboard>,
    pub histories: Map<HistoryKind, History>,
    pub keymaps: Map<KeymapKind, Keymap>,
    pub language_servers: Map<Filetype, LSPHandle>,
    pub filetree: Filetree,
}

impl Editor {
    fn new(runtime: TokioRuntime) -> Editor {
        let handle = runtime.editor_handle();
        // Spawn job runner
        let jobs_handle = runtime.block_on(spawn_job_runner(handle));
        let working_dir = env::current_dir().expect("Cannot get current working directory.");

        Editor {
            runtime,
            clients: Map::default(),
            draw_states: Map::default(),
            syntaxes: Syntaxes::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            job_broker: JobBroker::new(jobs_handle),
            hooks: Hooks::default(),
            keys: Vec::default(),
            is_running: true,
            config_dir: ConfigDirectory::default(),
            filetree: Filetree::new(&working_dir),
            working_dir,
            themes: Themes::default(),
            options: Options::default(),
            histories: Default::default(),
            clipboard: DefaultClipboard::new(),
            language_servers: Map::default(),
            keymaps: DefaultKeyMappings::keymaps(),
        }
    }

    pub fn configure(&mut self, mut opts: StartOptions) {
        if let Some(cd) = opts.config_dir.take() {
            if let Ok(cd) = cd.canonicalize() {
                let cd = ConfigDirectory::new(&cd);
                self.config_dir = cd;
                self.syntaxes = Syntaxes::new(&self.config_dir.filetype_dir());
                self.themes = Themes::new(&self.config_dir.theme_dir());
            }
        }

        if let Some(wd) = opts.working_dir.take() {
            if let Ok(wd) = wd.canonicalize() {
                let _ = self.change_working_dir(&wd);
            }
        }

        match config::read_config(&self.config_dir.config(), &self.working_dir) {
            Ok(toml) => {
                let Config {
                    editor,
                    window,
                    file,
                } = toml;
                self.options = editor;
                self.windows.set_default_options(window);
                self.buffers.set_default_options(file);
            }
            Err(e) => log::error!("Failed to read config: {e}"),
        }
    }

    /// Ran after the startup configuration is complete
    pub fn on_startup(&mut self) {
        self.themes.load_all();
    }

    pub fn buffers(&self) -> &Buffers {
        &self.buffers
    }

    pub fn buffers_mut(&mut self) -> &mut Buffers {
        &mut self.buffers
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

    /// Open an existing buffer in a window
    pub fn open_buffer(&mut self, id: ClientId, bid: BufferId) {
        let (_win, buf) = self.win_buf_mut(id);
        let old = buf.id;
        let is_modified = buf.is_modified();
        let is_used = self
            .windows
            .iter()
            .any(|(_, win)| win.buffer_id() == old || win.prev_buffer_id() == Some(old));

        if !is_modified && !is_used {
            run(self, id, Hook::BufDeletedPre(old));
            self.buffers.remove(old);
        }

        let (win, _buf) = self.win_buf_mut(id);
        win.open_buffer(bid);
        run(self, id, Hook::BufOpened(bid));
    }

    /// Create a new buffer from path
    pub fn create_buffer(&mut self, id: ClientId, path: impl AsRef<Path>) -> Result<BufferId> {
        let file = FileDescription::new(
            &path,
            self.options.big_file_threshold_bytes,
            &self.working_dir,
            &self.options.filetype,
        )?;
        let bid = self.buffers.new(file)?;
        run(self, id, Hook::BufCreated(bid));

        Ok(bid)
    }

    /// Open a file in window
    /// if the buffer already exists open that or create new if it doesnt
    pub fn open_file(&mut self, id: ClientId, path: impl AsRef<Path>) -> Result<()> {
        // Use existing if possible
        let bid = match self.buffers.find(&path) {
            Some(bid) => bid,
            None => self.create_buffer(id, &path)?,
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

        run(self, id, Hook::OnMessagePost);
    }

    fn create_context(&mut self, id: ClientId) -> EditorContext {
        let win = self.windows.get(id).expect("No window for {id}");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("No window for {id}");
        let theme = {
            let theme_name = &win.display_options().theme;
            self.themes
                .get(theme_name.as_str())
                .expect("Theme not present")
        };

        EditorContext {
            win,
            buf,
            theme,
            working_dir: &self.working_dir,
            filetree: &self.filetree,
            language_servers: &self.language_servers,
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

        // Redraw view
        win.redraw_view(buf);

        // Create draw state and send out initial draw
        let (draw, messages) = DrawState::new(self.create_context(id));
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
        let (win, buf) = self.win_buf_mut(id);
        if win.focus != Focus::Window {
            return;
        }

        // TODO keybindings
        match event.kind {
            MouseEventKind::ScrollDown => {
                win.scroll_down_n(buf, 3);
            }
            MouseEventKind::ScrollUp => {
                win.scroll_up_n(buf, 3);
            }
            MouseEventKind::ButtonDown(MouseButton::Left) => {
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
        let clients: Vec<ClientId> = self.clients.keys().cloned().collect();

        for cid in clients {
            self.redraw(cid);
        }
    }

    /// Redraw all windows that use the same buffer as `id`
    fn redraw(&mut self, id: ClientId) {
        // self.recalculate_syntax(id);

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

        let win = self.windows.get_mut(id).expect("No window for {id}");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("No window for {id}");
        let theme = {
            let theme_name = &win.display_options().theme;
            self.themes
                .get(theme_name.as_str())
                .expect("Theme not present")
        };

        win.redraw_view(buf);

        let ctx = EditorContext {
            win,
            buf,
            theme,
            working_dir: &self.working_dir,
            filetree: &self.filetree,
            language_servers: &self.language_servers,
        };

        let messages = draw.redraw(ctx);
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
        let keymap = self.focus_keymap(id);
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
                        buf.options.eol
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
            if let Err(e) = self.syntaxes.load(&ft) {
                log::error!("Failed to reload syntax: {e}");
            }
        }

        // Reload window
        let (win, buf) = self.win_buf_mut(id);
        win.reload();

        run(self, id, Hook::Reload);
    }

    pub fn paste_from_clipboard(&mut self, id: ClientId) {
        let Ok(text) = self.clipboard.paste() else {
            return;
        };
        let (win, buf) = self.win_buf_mut(id);
        let lines = paste_separate_cursor_lines(text.as_str());
        let clen = win.cursors.cursors().len();
        let llen = lines.len();
        let bid = buf.id;

        if clen == llen {
            win.insert_to_each_cursor(buf, lines);
        } else {
            win.insert_at_cursors(buf, &text);
        }

        run(self, id, Hook::BufChanged(bid));
    }

    pub fn copy_to_clipboard(&mut self, id: ClientId) {
        let (win, buf) = self.win_buf_mut(id);
        let mut lines = vec![];
        for cursor in win.cursors.cursors_mut() {
            if let Some(sel) = cursor.selection() {
                let text = String::from(&buf.slice(sel));
                lines.push(text);
            }
        }

        let line = copy_cursors_to_lines(lines, buf.options.eol);
        self.clipboard.copy(&line);
    }

    pub fn prompt_history_next(&mut self, id: ClientId) {
        let win = self.windows.get_mut(id).expect("No window found");
        if let Some(kind) = win.prompt.history() {
            let history = self.histories.entry(kind).or_default();
            win.prompt.history_next(history);
        }
    }

    pub fn prompt_history_prev(&mut self, id: ClientId) {
        let win = self.windows.get_mut(id).expect("No window found");
        if let Some(kind) = win.prompt.history() {
            let history = self.histories.entry(kind).or_default();
            win.prompt.history_prev(history);
        }
    }

    pub fn keymap(&self) -> &Keymap {
        self.keymaps
            .get(&KeymapKind::Window)
            .expect("No keymap for window")
    }

    /// Return the currently focused elements keymap
    pub fn focus_keymap(&self, id: ClientId) -> &Keymap {
        use Focus::*;

        let (win, buf) = self.win_buf(id);
        let kind = match win.focus {
            Search | Prompt => win.prompt.keymap_kind(),
            Window => KeymapKind::Window,
            Completion => KeymapKind::Completion,
            Filetree => KeymapKind::Filetree,
            Locations => KeymapKind::Locations,
        };

        self.keymaps
            .get(&kind)
            .expect("No keymap found for kind: {kind:?}")
    }

    pub fn change_working_dir(&mut self, path: &Path) -> Result<()> {
        self.working_dir = path.into();
        self.filetree = Filetree::new(&self.working_dir);
        Ok(())
    }

    pub fn lsp_handle_for(&self, id: ClientId) -> Option<&LSPHandle> {
        let (win, buf) = self.win_buf(id);
        let ft = buf.filetype.as_ref()?;
        self.language_servers.get(ft)
    }

    pub fn lsp_handle_for_mut(&mut self, id: ClientId) -> Option<&mut LSPHandle> {
        let win = self.windows.get(id)?;
        let buf = self.buffers.get_mut(win.buffer_id())?;
        let ft = buf.filetype.as_ref()?;
        self.language_servers.get_mut(ft)
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

    // let framerate = Duration::from_millis(1000 / 30);
    // let mut redraw = Instant::now();
    // let mut was_previously_redrawn = false;

    while editor.is_running {
        // match recv.recv_timeout(framerate) {
        match recv.recv() {
            Ok(msg) => {
                // was_previously_redrawn = false;

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

                editor.redraw_all();
                // let now = Instant::now();
                // if now.duration_since(redraw) > framerate {
                //     was_previously_redrawn = true;
                //     redraw = Instant::now();
                //     editor.redraw_all();
                // }
            }
            Err(_) => {
                // if !was_previously_redrawn {
                //     was_previously_redrawn = true;
                //     redraw = Instant::now();
                //     editor.redraw_all();
                // }
            }
        }
    }

    Ok(())
}
