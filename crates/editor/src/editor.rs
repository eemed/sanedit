pub(crate) mod buffers;
pub(crate) mod caches;
pub(crate) mod clipboard;
pub(crate) mod config;
pub(crate) mod file_description;
pub(crate) mod filetree;
pub(crate) mod filetype;
pub(crate) mod hooks;
pub(crate) mod job_broker;
pub(crate) mod keymap;
pub(crate) mod lsp;
pub(crate) mod snippets;
pub(crate) mod syntax;
pub(crate) mod themes;
pub(crate) mod windows;

use caches::Caches;
use file_description::FileDescription;
use filetype::Filetypes;
use keymap::KeymapResult;
use keymap::Layer;
use keymap::LayerKey;
use rustc_hash::FxHashMap;
use sanedit_core::Filetype;
use sanedit_messages::key;
use sanedit_messages::key::KeyEvent;
use sanedit_messages::redraw::Size;
use sanedit_messages::ClientMessage;
use sanedit_messages::Command;
use sanedit_messages::Message;
use sanedit_messages::MouseButton;
use sanedit_messages::MouseEvent;
use sanedit_messages::MouseEventKind;
use sanedit_server::spawn_job_runner;
use sanedit_server::ClientHandle;
use sanedit_server::ClientId;
use sanedit_server::FromJobs;
use sanedit_server::StartOptions;
use sanedit_server::ToEditor;
use strum::IntoEnumIterator;
use windows::Mode;

use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use tokio::io;

use anyhow::Result;

use crate::actions;
use crate::actions::cursors;
use crate::actions::hooks::run;
use crate::common::matcher::Choice;
use crate::draw::DrawState;
use crate::draw::EditorContext;
use crate::editor::buffers::Buffer;
use crate::editor::config::Config;
use crate::editor::hooks::Hook;
use crate::editor::windows::Focus;
use crate::runtime::TokioRuntime;
use sanedit_core::copy_cursors_to_lines;
use sanedit_core::paste_separate_cursor_lines;
use sanedit_core::ConfigDirectory;

use self::buffers::BufferId;
use self::buffers::Buffers;
use self::clipboard::Clipboard;
use self::clipboard::DefaultClipboard;
use self::config::EditorConfig;
use self::hooks::Hooks;
use self::job_broker::JobBroker;
use self::keymap::Keymaps;

use self::filetree::Filetree;
use self::lsp::LSP;
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
    is_running: bool,
    working_dir: PathBuf,

    pub windows: Windows,
    pub buffers: Buffers,
    pub _runtime: TokioRuntime,
    pub themes: Themes,
    pub config_dir: ConfigDirectory,
    pub syntaxes: Syntaxes,
    pub filetypes: Filetypes,
    pub job_broker: JobBroker,
    pub hooks: Hooks,
    pub clipboard: Box<dyn Clipboard>,
    pub histories: Map<HistoryKind, History>,
    pub keymaps: Keymaps,
    pub language_servers: Map<Filetype, LSP>,
    pub filetree: Filetree,
    pub config: Config,
    pub caches: Caches,
}

impl Editor {
    fn new(runtime: TokioRuntime) -> Editor {
        let handle = runtime.editor_handle();
        // Spawn job runner
        let jobs_handle = runtime.block_on(spawn_job_runner(handle));
        let working_dir = env::current_dir().expect("Cannot get current working directory.");
        let config_dir = ConfigDirectory::default();
        let config = Config::default();
        let caches = Caches::new(&config);

        Editor {
            _runtime: runtime,
            clients: Map::default(),
            draw_states: Map::default(),
            syntaxes: Syntaxes::new(),
            filetypes: Filetypes::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            job_broker: JobBroker::new(jobs_handle),
            hooks: Hooks::default(),
            is_running: true,
            config_dir,
            filetree: Filetree::new(&working_dir),
            working_dir,
            themes: Themes::default(),
            histories: Default::default(),
            clipboard: DefaultClipboard::new(),
            language_servers: Map::default(),
            keymaps: Keymaps::default(),
            config,
            caches,
        }
    }

    pub fn configure(&mut self, mut opts: StartOptions) {
        if let Some(cd) = opts.config_dir.take() {
            log::info!("Config directory: {cd:?}");
            if let Ok(cd) = cd.canonicalize() {
                self.config_dir = ConfigDirectory::new(&cd);
                self.syntaxes = Syntaxes::new();
                self.themes = Themes::new(self.config_dir.theme_dir());
            }
        }

        if let Some(wd) = opts.working_dir.take() {
            if let Ok(wd) = wd.canonicalize() {
                let _ = self.change_working_dir(&wd);
            }
        }

        self.reload_config();
    }

    fn reload_config(&mut self) {
        self.config = Config::new(&self.config_dir.config(), &self.working_dir);
        self.caches = Caches::new(&self.config);
        self.configure_keymap();
    }

    fn configure_keymap(&mut self) {
        self.keymaps = Keymaps::default();

        for (name, kmlayer) in &self.config.keymaps {
            let key = {
                let mode = Mode::iter().find(|m| m.as_ref() == name);
                let focus = Focus::iter().find(|f| f.as_ref() == name);

                match (mode, focus) {
                    (Some(mode), _) => LayerKey {
                        mode,
                        focus: Focus::Window,
                    },
                    (_, Some(focus)) => LayerKey {
                        mode: Mode::Normal,
                        focus,
                    },
                    _ => continue,
                }
            };
            // Insert new layer
            let layer = kmlayer.to_layer(name);
            self.keymaps.insert(key, layer);
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
        log::info!("Quit client: {id:?}");
        self.send_to_client(id, ClientMessage::Bye);

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
        let client_ids: Vec<ClientId> = self.clients.keys().copied().collect();
        for id in client_ids {
            log::info!("Quit to {:?}", id);
            // Dont care about errors here we are quitting anyway
            self.send_to_client(id, ClientMessage::Bye);
        }
        self.is_running = false;
    }

    fn on_client_connected(&mut self, handle: ClientHandle) {
        log::info!(
            "Client connected: {:?}, id: {:?}",
            handle.connection_info(),
            handle.id()
        );
        self.clients.insert(handle.id(), handle);
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

        run(self, id, Hook::BufLeave(old));

        let (win, _buf) = self.win_buf_mut(id);
        win.open_buffer(bid);
        run(self, id, Hook::BufEnter(bid));
    }

    /// Create a new buffer from path
    pub fn create_buffer(&mut self, id: ClientId, path: impl AsRef<Path>) -> Result<BufferId> {
        let file = FileDescription::new(path, &self.config, &self.filetypes)?;
        let config = file
            .filetype
            .as_ref()
            .map(|ft| self.filetypes.get(&ft))
            .flatten()
            .map(|ftconfig| ftconfig.buffer.clone())
            .unwrap_or_default();
        let bid = self.buffers.new(file, config)?;
        run(self, id, Hook::BufCreated(bid));

        Ok(bid)
    }

    /// Open a file in window
    /// if the buffer already exists open that or create new if it doesnt
    pub fn open_file(&mut self, id: ClientId, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let path = if path.is_relative() {
            self.working_dir.join(path)
        } else {
            path.to_path_buf()
        };

        // Use existing if possible
        let bid = match self.buffers.find(&path) {
            Some(bid) => bid,
            None => self.create_buffer(id, &path)?,
        };
        let (win, _buf) = self.win_buf(id);
        if win.buffer_id() == bid {
            return Ok(());
        }

        self.open_buffer(id, bid);

        Ok(())
    }

    pub fn send_to_client(&mut self, id: ClientId, msg: ClientMessage) {
        if let Some(client) = self.clients.get_mut(&id) {
            if let Err(_e) = client.send(msg.into()) {
                log::info!(
                    "Server failed to send to client {:?}, removing from client map",
                    id
                );
                self.clients.remove(&id);
            }
        }
    }

    fn handle_message(&mut self, id: ClientId, msg: Message) {
        if let Message::Hello(size) = msg {
            self.handle_hello(id, size);
            return;
        }

        run(self, id, Hook::OnMessagePre);

        match msg {
            Message::KeyEvent(key_event) => self.handle_key_event(id, key_event),
            Message::MouseEvent(mouse_event) => self.handle_mouse_event(id, mouse_event),
            Message::Resize(size) => self.handle_resize(id, size),
            Message::Command(cmd) => self.handle_command(id, cmd),
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
            let theme_name = &win.config.theme;
            self.themes.get(theme_name).expect("Theme not present")
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
        self.windows
            .new_window(id, bid, size.width, size.height, self.config.window.clone());

        let win = self.windows.get_mut(id).expect("Window not present");
        let buf = self.buffers.get(bid).expect("Buffer not present");
        let theme = {
            let theme_name = &win.config.theme;
            self.themes
                .get(theme_name)
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

        run(self, id, Hook::BufEnter(bid));
        run(self, id, Hook::ModeEnter);
    }

    fn handle_resize(&mut self, id: ClientId, size: Size) {
        let (win, buf) = self.win_buf_mut(id);
        win.resize(size, buf);
    }

    fn handle_command(&mut self, id: ClientId, cmd: Command) {
        match cmd {
            Command::OpenFile(path_buf) => {
                let _ = self.open_file(id, &path_buf);
            }
        }
    }

    fn handle_mouse_event(&mut self, id: ClientId, event: MouseEvent) {
        let (win, buf) = self.win_buf_mut(id);
        if win.focus() != Focus::Window {
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
                if event.mods & key::CONTROL != 0 {
                    cursors::new_to_point(self, id, event.point);
                } else if event.mods == 0 {
                    cursors::goto_position(self, id, event.point);
                }
            }
            _ => {}
        }
    }

    fn redraw_all(&mut self) {
        let clients: Vec<ClientId> = self.clients.keys().cloned().collect();
        let mut drawn = HashSet::new();

        for cid in clients {
            if drawn.contains(&cid) {
                continue;
            }

            let drawn_clients = self.redraw(cid);
            drawn.extend(drawn_clients);
        }
    }

    /// Redraw all windows that use the same buffer as `id`
    fn redraw(&mut self, id: ClientId) -> Vec<ClientId> {
        let mut drawn = vec![];
        // Editor is closed or client is closed
        if !self.is_running || !self.clients.contains_key(&id) {
            return drawn;
        }

        if let Some(bid) = self.windows.bid(id) {
            for cid in self.windows.find_clients_with_buf(bid) {
                self.redraw_client(cid);
                drawn.push(cid);
            }
        }

        drawn
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
            let theme_name = &win.config.theme;
            self.themes.get(theme_name).expect("Theme not present")
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
        log::info!("KeyEvent '{event}'");
        use sanedit_messages::key::Key::*;

        // Add key to buffer
        let (win, _buf) = self.win_buf_mut(id);
        win.push_key(event);

        run(self, id, Hook::KeyPressedPre);

        // If next key handler specified
        let (win, _buf) = self.win_buf_mut(id);
        if let Some(handler) = win.next_key_handler.take() {
            let event = win.clear_keys().pop().unwrap();
            (handler.0)(self, id, event);
            return;
        }

        // Handle key bindings
        let events;
        match self.mapped_action(id) {
            KeymapResult::Matched(action) => {
                action.execute(self, id);
                // We may have removed the window
                if let Some(win) = self.windows.get_mut(id) {
                    win.clear_keys();
                }
                return;
            }
            KeymapResult::Pending(action) => {
                if let Some(action) = action {
                    action.execute(self, id);
                }
                return;
            }
            KeymapResult::NotFound => {
                let (win, _buf) = self.win_buf_mut(id);
                events = win.clear_keys();

                if win.focus == Focus::Window && win.mode != Mode::Insert {
                    return;
                }
            }
        }

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
                        buf.config.eol
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
        // Reload config
        self.reload_config();

        // Reload theme
        let (win, _buf) = self.win_buf(id);
        let theme = win.config.theme.to_string();
        if let Ok(theme) = self.themes.load(&theme).cloned() {
            self.send_to_client(id, ClientMessage::Theme(theme))
        }

        // Reload filetype
        let (_win, buf) = self.win_buf(id);
        if let Some(ft) = buf.filetype.clone() {
            self.load_filetype(&ft, true);
        }

        // Reload window
        let (win, _buf) = win_buf!(self, id);
        win.reload();

        run(self, id, Hook::Reload);
    }

    pub fn paste_from_clipboard(&mut self, id: ClientId) {
        let Ok(text) = self.clipboard.paste() else {
            return;
        };

        let (win, _buf) = self.win_buf(id);
        let lines = paste_separate_cursor_lines(text.as_str());
        let single_with_eol = lines.len() == 1 && lines[0].1 && win.cursors.len() == 1;
        let multicursor_match = win.cursors.len() == lines.len();

        if single_with_eol || !multicursor_match {
            self.paste_on_line_below(id, lines);
        } else {
            self.paste_inline(id, lines);
        }
    }

    fn paste_on_line_below(&mut self, id: ClientId, lines: Vec<(String, bool)>) {
        let (win, buf) = self.win_buf_mut(id);
        let bid = buf.id;
        let text = {
            let mut result: Vec<String> = vec![];
            for (mut line, _) in lines {
                line.push_str(buf.config.eol.as_str());
                result.push(line);
            }
            result.join("")
        };

        let res = win.insert_at_cursors_next_line(buf, &text);

        if res.is_ok() {
            run(self, id, Hook::BufChanged(bid));
        }
    }

    // Paste to current cursors
    fn paste_inline(&mut self, id: ClientId, lines: Vec<(String, bool)>) {
        let (win, buf) = self.win_buf_mut(id);
        let clen = win.cursors.cursors().len();
        let llen = lines.len();
        let bid = buf.id;

        let res = if clen == llen {
            let lines = lines.into_iter().map(|(line, _)| line).collect();
            win.insert_to_each_cursor(buf, lines)
        } else {
            let text = lines
                .into_iter()
                .map(|(mut line, eol)| {
                    if eol {
                        line.push_str(buf.config.eol.as_str());
                    }
                    line
                })
                .collect::<Vec<String>>()
                .join("");
            win.insert_at_cursors(buf, &text)
        };

        if res.is_ok() {
            run(self, id, Hook::BufChanged(bid));
        }
    }

    pub fn copy_line_to_clipboard(&mut self, id: ClientId) {
        let (win, buf) = self.win_buf_mut(id);
        let mut lines = vec![];

        for range in win.cursor_lines(buf) {
            let text = String::from(&buf.slice(range));
            lines.push(text);
        }

        let line = copy_cursors_to_lines(lines, buf.config.eol);
        self.clipboard.copy(&line);
    }

    pub fn copy_to_eol_to_clipboard(&mut self, id: ClientId) {
        let (win, buf) = self.win_buf_mut(id);
        let mut lines = vec![];

        for range in win.cursors_to_eol(buf) {
            let text = String::from(&buf.slice(range));
            lines.push(text);
        }

        let line = copy_cursors_to_lines(lines, buf.config.eol);
        self.clipboard.copy(&line);
    }

    pub fn copy_to_clipboard(&mut self, id: ClientId) {
        let (win, buf) = self.win_buf(id);
        let mut lines = vec![];
        for cursor in win.cursors.cursors() {
            if let Some(sel) = cursor.selection() {
                let text = String::from(&buf.slice(sel));
                lines.push(text);
            }
        }

        let line = copy_cursors_to_lines(lines, buf.config.eol);
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

    pub fn layer(&self, id: ClientId) -> Option<&Layer> {
        let win = self.windows.get(id).expect("No window found");
        let key = win.layer();
        self.keymaps.get_layer(&key)
    }

    /// Return the currently focused elements keymap
    pub fn mapped_action(&self, id: ClientId) -> KeymapResult {
        let (win, _buf) = self.win_buf(id);
        let kmap = &self.keymaps;
        let key = win.layer();
        kmap.get(&key, &win.keys())
    }

    pub fn change_working_dir(&mut self, path: &Path) -> Result<()> {
        self.working_dir = path.into();
        self.filetree = Filetree::new(&self.working_dir);
        Ok(())
    }

    pub fn has_syntax(&self, id: ClientId) -> bool {
        let (_win, buf) = self.win_buf(id);
        if let Some(ref ft) = buf.filetype {
            return self.syntaxes.contains_key(ft);
        }

        false
    }

    pub fn has_lsp(&self, id: ClientId) -> bool {
        self.lsp_for(id).is_some()
    }

    pub fn lsp_for(&self, id: ClientId) -> Option<&LSP> {
        let (_win, buf) = self.win_buf(id);
        let ft = buf.filetype.as_ref()?;
        self.language_servers.get(ft)
    }

    pub fn load_filetype(&mut self, ft: &Filetype, reload: bool) {
        self.load_filetype_syntax(ft, reload);
        self.load_filetype_config(ft, reload);
    }

    fn load_filetype_config(&mut self, ft: &Filetype, reload: bool) {
        let dir = self.config_dir.filetype_dir();
        let path = PathBuf::from(ft.as_str()).join("config.toml");
        if let Some(path) = dir.find(&path) {
            let result = if reload {
                self.filetypes.reload(ft, &path)
            } else {
                self.filetypes.load(ft, &path)
            };
            if let Err(e) = result {
                log::error!("Failed to load filetype config for {}: {e}", ft.as_str());
            }
        }
    }

    fn load_filetype_syntax(&mut self, ft: &Filetype, reload: bool) {
        let dir = self.config_dir.filetype_dir();
        let path = PathBuf::from(ft.as_str()).join("syntax.peg");
        if let Some(path) = dir.find(&path) {
            let result = if reload {
                self.syntaxes.reload(ft, &path)
            } else {
                self.syntaxes.load(ft, &path)
            };
            if let Err(e) = result {
                log::error!("Failed to load syntax for {}: {e}", ft.as_str());
            }
        }
    }

    pub fn get_snippets(&self, id: ClientId) -> Vec<Arc<Choice>> {
        let mut result = self.config.snippets_as_choices();

        let win = self.windows.get(id).expect("No window for {id}");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("No window for {id}");
        let Some(ft) = buf.filetype.as_ref() else {
            return result;
        };

        let Some(ftconfig) = self.filetypes.get(&ft) else {
            return result;
        };

        result.extend(ftconfig.snippets_as_choices());
        result
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
