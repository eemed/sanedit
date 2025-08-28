pub(crate) mod buffers;
pub(crate) mod caches;
pub(crate) mod clipboard;
pub(crate) mod config;
pub(crate) mod file_description;
pub(crate) mod filetree;
pub(crate) mod hooks;
pub(crate) mod ignore;
pub(crate) mod job_broker;
pub(crate) mod keymap;
pub(crate) mod language;
pub(crate) mod lsp;
pub(crate) mod snippets;
pub(crate) mod syntax;
pub(crate) mod themes;
pub(crate) mod windows;

use anyhow::bail;
use caches::Caches;
use config::ProjectConfig;
use crossbeam::channel::Sender;
use file_description::FileDescription;
use ignore::Ignore;
use keymap::KeymapResult;
use keymap::Layer;
use language::Languages;
use rustc_hash::FxHashMap;
use sanedit_core::Language;
use sanedit_messages::key::KeyEvent;
use sanedit_messages::redraw::Size;
use sanedit_messages::ClientMessage;
use sanedit_messages::Command;
use sanedit_messages::Element;
use sanedit_messages::Message;
use sanedit_messages::MouseButton;
use sanedit_messages::MouseEvent;
use sanedit_messages::MouseEventKind;
use sanedit_server::spawn_job_runner;
use sanedit_server::Address;
use sanedit_server::ClientHandle;
use sanedit_server::ClientId;
use sanedit_server::FromEditorSharedMessage;
use sanedit_server::FromJobs;
use sanedit_server::ServerOptions;
use sanedit_server::ToEditor;
use tokio::runtime::Runtime;
use windows::Mode;
use windows::MouseClick;
use windows::Zone;

use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io;

use anyhow::Result;

use crate::actions;
use crate::actions::hooks::run;
use crate::actions::mouse;
use crate::actions::window::focus_with_mode;
use crate::actions::window::goto_other_buffer;
use crate::common::Choice;
use crate::draw::DrawState;
use crate::draw::EditorContext;
use crate::editor::buffers::Buffer;
use crate::editor::config::Config;
use crate::editor::hooks::Hook;
use crate::editor::windows::Focus;
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
    listen_address: Address,

    pub ignore: Ignore,
    pub windows: Windows,
    pub buffers: Buffers,
    pub _tokio_runtime: Runtime,
    pub themes: Themes,
    pub config_dir: ConfigDirectory,
    pub syntaxes: Syntaxes,
    pub languages: Languages,
    pub job_broker: JobBroker,
    pub hooks: Hooks,
    pub clipboard: Box<dyn Clipboard>,
    pub histories: Map<HistoryKind, History>,
    pub keymaps: Keymaps,
    pub language_servers: Map<Language, LSP>,
    pub filetree: Filetree,
    pub config: Config,
    pub project_config: ProjectConfig,
    pub caches: Caches,
}

impl Editor {
    fn new(runtime: Runtime, internal_chan: Sender<ToEditor>, opts: ServerOptions) -> Editor {
        // Spawn job runner
        let jobs_handle = runtime.block_on(spawn_job_runner(internal_chan));
        let working_dir = opts
            .working_dir
            .map(|dir| dir.canonicalize().ok())
            .flatten()
            .unwrap_or_else(|| env::current_dir().expect("Cannot get current working directory."));
        let config_dir = opts
            .config_dir
            .map(|dir| dir.canonicalize().ok())
            .flatten()
            .map(|dir| ConfigDirectory::new(&dir))
            .unwrap_or_default();
        let config = Config::new(&config_dir.config(), &working_dir);
        let caches = Caches::new(&config);
        let project_config = ProjectConfig::new(&working_dir);
        let ignore = Ignore::new(&working_dir, &config, &project_config);

        Editor {
            _tokio_runtime: runtime,
            listen_address: opts.addr.clone(),
            clients: Map::default(),
            draw_states: Map::default(),
            syntaxes: Syntaxes::new(),
            languages: Languages::default(),
            windows: Windows::default(),
            buffers: Buffers::default(),
            job_broker: JobBroker::new(jobs_handle),
            hooks: Hooks::default(),
            is_running: true,
            themes: Themes::new(config_dir.theme_dir()),
            config_dir,
            filetree: Filetree::new(&working_dir),
            project_config,
            working_dir,
            histories: Default::default(),
            clipboard: DefaultClipboard::new(),
            language_servers: Map::default(),
            keymaps: Keymaps::from_config(&config),
            config,
            caches,
            ignore,
        }
    }

    fn reload_config(&mut self) {
        self.project_config = ProjectConfig::new(&self.working_dir);
        self.config = Config::new(&self.config_dir.config(), &self.working_dir);
        self.ignore = Ignore::new(&self.working_dir, &self.config, &self.project_config);
        self.caches = Caches::new(&self.config);
        self.keymaps = Keymaps::from_config(&self.config);
    }

    pub fn listen_address(&self) -> &Address {
        &self.listen_address
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
        let win = self.windows.get(id).expect("no win for cliend id");
        let bid = win.buffer_id();
        let buf = self.buffers.get(bid).expect("no buffer for buffer id");
        (win, buf)
    }

    pub fn win_buf_mut(&mut self, id: ClientId) -> (&mut Window, &mut Buffer) {
        let win = self.windows.get_mut(id).expect("no win for cliend id");
        let bid = win.buffer_id();
        let buf = self.buffers.get_mut(bid).expect("no buffer for buffer id");
        (win, buf)
    }

    pub fn windows(&self) -> &Windows {
        &self.windows
    }

    pub fn quit_client(&mut self, id: ClientId) {
        log::info!("Quit client: {id:?}");
        self.send_to_client(id, ClientMessage::Bye.into());

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

    pub fn is_last_client(&self) -> bool {
        self.clients.len() == 1
    }

    pub fn quit(&mut self) {
        let client_ids: Vec<ClientId> = self.clients.keys().copied().collect();
        for id in client_ids {
            log::info!("Quit to {:?}", id);
            // Dont care about errors here we are quitting anyway
            self.send_to_client(id, ClientMessage::Bye.into());
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
        if self.buffers.get(bid).is_none() {
            return;
        }

        let (_win, buf) = self.win_buf_mut(id);
        let old = buf.id;

        // TODO buffer deletion when needed? if ever?

        run(self, id, Hook::BufLeave(old));

        let (_win, buf) = self.win_buf_mut(id);
        if buf.remove_on_exit {
            run(self, id, Hook::BufDeletedPre(old));
            self.buffers.remove(old);
        }

        if let Some(buf) = self.buffers.get(bid) {
            let win = self.windows.get_mut(id).unwrap();
            win.open_buffer(buf);
            run(self, id, Hook::BufEnter(bid));
        }
    }

    /// Create a new buffer from path
    pub fn create_buffer(&mut self, id: ClientId, path: impl AsRef<Path>) -> Result<BufferId> {
        let file = FileDescription::new(path, &self.config)?;
        if let Some(lang) = &file.language {
            self.load_language(lang, false);
        }
        let config = file
            .language
            .as_ref()
            .map(|lang| self.languages.get(&lang))
            .flatten()
            .map(|lang_config| lang_config.buffer.clone())
            .unwrap_or_else(|| {
                // If eol and indent are detected automatically they will override using the hook
                self.config.buffer.clone()
            });

        let bid = self.buffers.new(file, config)?;
        run(self, id, Hook::BufCreated(bid));

        Ok(bid)
    }

    pub fn remove_buffer(&mut self, id: ClientId, bid: BufferId) -> Result<()> {
        if self.buffers.get(bid).is_none() {
            bail!("No such buffer {bid:?}");
        }

        let clients = self.windows.find_clients_with_buf(bid);
        for client in clients {
            goto_other_buffer(self, client);
        }

        run(self, id, Hook::BufDeletedPre(bid));
        self.buffers.remove(bid);
        Ok(())
    }

    /// Open a file in window
    /// if the buffer already exists open that or create new if it doesnt
    pub fn open_file(&mut self, id: ClientId, path: impl AsRef<Path>) -> Result<()> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().into());
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

    pub fn send_to_client(&mut self, id: ClientId, msg: FromEditorSharedMessage) {
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
        if let Message::Hello {
            color_count,
            size,
            parent,
        } = msg
        {
            self.handle_hello(id, color_count, size, parent);
            return;
        }

        run(self, id, Hook::OnMessagePre);

        match msg {
            Message::KeyEvent(key_event) => self.handle_key_event(id, key_event),
            Message::MouseEvent(mouse_event) => self.handle_mouse_event(id, mouse_event),
            Message::Resize(size) => self.handle_resize(id, size),
            Message::Command(cmd) => self.handle_command(id, cmd),
            Message::FocusGained => run(self, id, Hook::Focus),
            Message::FocusLost => run(self, id, Hook::Unfocus),
            _ => {}
        }

        run(self, id, Hook::OnMessagePost);
    }

    fn create_context(&mut self, id: ClientId) -> EditorContext<'_> {
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

    fn handle_hello(
        &mut self,
        id: ClientId,
        color_count: usize,
        size: Size,
        parent: Option<usize>,
    ) {
        // Create buffer and window
        let (bid, config, view_offset) = parent
            .map(|parent| {
                let win = self
                    .windows
                    .get_mut(ClientId::temporary(parent))
                    .expect("Window not present");
                let bid = win.buffer_id();
                let config = win.config.clone();
                let offset = win.view().start();
                (bid, config, offset)
            })
            .unwrap_or_else(|| {
                let bid = self.buffers.insert(Buffer::default());
                let config = self.config.window.clone();
                (bid, config, 0)
            });
        self.windows
            .new_window(id, bid, size.width, size.height, config);
        let buf = self.buffers.get(bid).expect("Buffer not present");
        let win = self.windows.get_mut(id).expect("Window not present");
        win.goto_view_offset(view_offset, buf);

        let theme = {
            log::info!("color count: {color_count}");
            let name = if color_count <= 16 {
                "less"
            } else {
                &win.config.theme
            };
            self.themes.get(name).expect("Theme not present").clone()
        };

        // Redraw view
        win.redraw_view(buf);

        // Create draw state and send out initial draw
        let (draw, messages) = DrawState::new(self.create_context(id));
        self.draw_states.insert(id, draw);

        self.send_to_client(id, ClientMessage::Hello { id: id.as_usize() }.into());
        self.send_to_client(id, ClientMessage::Theme(theme).into());
        for msg in messages {
            self.send_to_client(id, msg);
        }
        self.send_to_client(id, ClientMessage::Flush.into());

        run(self, id, Hook::BufEnter(bid));
        run(self, id, Hook::ModeEnter);
    }

    fn handle_resize(&mut self, id: ClientId, size: Size) {
        let (win, buf) = self.win_buf_mut(id);
        win.resize(size, buf);
    }

    fn handle_command(&mut self, id: ClientId, cmd: Command) {
        match cmd {
            Command::OpenFile { path, language } => {
                if let Err(e) = self.open_file(id, &path) {
                    log::error!("Failed to open file: {e}");
                    return;
                }

                if let Some(lang) = language {
                    let language = Language::new(&lang);
                    self.load_language(&language, false);
                    let (_win, buf) = self.win_buf_mut(id);
                    buf.language = Some(language);
                }
            }
            Command::ReadStdin { bytes, language } => {
                if let Ok(buf) = Buffer::from_reader(std::io::Cursor::new(bytes)) {
                    let bid = self.buffers.insert(buf);
                    run(self, id, Hook::BufCreated(bid));
                    self.open_buffer(id, bid);

                    if let Some(lang) = language {
                        let language = Language::new(&lang);
                        self.load_language(&language, false);
                        let buf = self.buffers.get_mut(bid).unwrap();
                        buf.language = Some(language);
                    }
                }
            }
        }
    }

    fn handle_mouse_event(&mut self, id: ClientId, event: MouseEvent) {
        match event.element {
            Element::Filetree => match event.kind {
                MouseEventKind::ButtonDown(MouseButton::Left) => {
                    focus_with_mode(self, id, Focus::Filetree, Mode::Normal);
                    let (win, _buf) = self.win_buf_mut(id);
                    win.ft_view.mouse.on_click(event.point);
                    match win.ft_view.mouse.clicks() {
                        MouseClick::Single => {
                            win.ft_view.selection = event.point.y;
                        }
                        MouseClick::Double => {
                            actions::filetree::goto_ft_entry.execute(self, id);
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            Element::Locations => match event.kind {
                MouseEventKind::ButtonDown(MouseButton::Left) => {
                    focus_with_mode(self, id, Focus::Locations, Mode::Normal);
                    let (win, _buf) = self.win_buf_mut(id);
                    let mouse = &mut win.locations.extra.mouse;
                    mouse.on_click(event.point);
                    match mouse.clicks() {
                        MouseClick::Single => {
                            win.locations.select(event.point.y);
                        }
                        MouseClick::Double => {
                            actions::locations::goto_loc_entry.execute(self, id);
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            Element::Window => {
                let (win, _buf) = self.win_buf_mut(id);
                if win.focus != Focus::Window {
                    focus_with_mode(self, id, Focus::Filetree, Mode::Normal);
                }
                let (win, buf) = self.win_buf_mut(id);
                match event.kind {
                    MouseEventKind::ScrollDown => win.scroll_down_n(buf, 3),
                    MouseEventKind::ScrollUp => win.scroll_up_n(buf, 3),
                    MouseEventKind::ButtonDown(MouseButton::Left) => {
                        mouse::on_button_down_left_click(self, id, event);
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        mouse::on_drag(self, id, event.point);
                    }
                    _ => {}
                }
            }
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
                self.send_to_client(id, msg);
            }
            self.send_to_client(id, ClientMessage::Flush.into());
        }
    }

    fn handle_key_event(&mut self, id: ClientId, event: KeyEvent) {
        log::info!("KeyEvent '{event}'");
        use sanedit_messages::key::Key::*;

        let (win, _buf) = self.win_buf_mut(id);
        if let Some(game) = win.game.as_mut() {
            if game.handle_input(event) {
                win.game = None;
            }

            return;
        }

        // Add key to buffer
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
            Stopped(id) => {
                log::info!("Job {id} stopped.");
                if let Some(prog) = self.job_broker.get(id) {
                    prog.on_stop(self);
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
            self.send_to_client(id, ClientMessage::Theme(theme).into())
        }

        // Reload language
        let (_win, buf) = self.win_buf(id);
        if let Some(lang) = buf.language.clone() {
            self.load_language(&lang, true);
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
            win.view_to_around_cursor_zone(buf, Zone::Middle);
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
        win.view_to_around_cursor_zone(buf, Zone::Middle);

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

        if lines.len() == 1 {
            lines[0].push_str(buf.config.eol.as_str());
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
        std::env::set_current_dir(path)?;
        self.working_dir = path.into();
        self.filetree = Filetree::new(&self.working_dir);
        Ok(())
    }

    pub fn has_syntax(&self, id: ClientId) -> bool {
        let (_win, buf) = self.win_buf(id);
        if let Some(ref lang) = buf.language {
            return self.syntaxes.contains_key(lang);
        }

        false
    }

    pub fn has_lsp(&self, id: ClientId) -> bool {
        self.lsp_for(id).is_some()
    }

    pub fn lsp_for(&self, id: ClientId) -> Option<&LSP> {
        let (_win, buf) = self.win_buf(id);
        let lang = buf.language.as_ref()?;
        self.language_servers.get(lang)
    }

    pub fn load_language(&mut self, lang: &Language, reload: bool) {
        self.load_language_syntax(lang, reload);
        self.load_language_config(lang, reload);
    }

    fn load_language_config(&mut self, lang: &Language, reload: bool) {
        let dir = self.config_dir.lang_dir();
        let path = PathBuf::from(lang.as_str()).join("config.toml");
        if let Some(path) = dir.find(&path) {
            let result = if reload {
                self.languages.reload(lang, &path)
            } else {
                self.languages.load(lang, &path)
            };
            match result {
                Ok(()) => {
                    log::debug!("Loaded language config for {}", lang.as_str());
                }
                Err(e) => {
                    log::error!("Failed to load language config for {}: {e}", lang.as_str());
                }
            }
        }
    }

    fn load_language_syntax(&mut self, lang: &Language, reload: bool) {
        let loader = self.syntaxes.loader(
            self.config_dir.lang_dir(),
            self.config.editor.language_detect.clone(),
        );
        loader.load_language(lang, reload);
    }

    pub fn get_snippets(&self, id: ClientId) -> Vec<Arc<Choice>> {
        let mut result = self.config.snippets_as_choices();

        let win = self.windows.get(id).expect("No window for {id}");
        let buf = self
            .buffers
            .get(win.buffer_id())
            .expect("No window for {id}");
        let Some(lang) = buf.language.as_ref() else {
            return result;
        };

        let Some(langconfig) = self.languages.get(&lang) else {
            return result;
        };

        result.extend(langconfig.snippets_as_choices());
        result
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) fn main_loop(
    runtime: Runtime,
    recv: crossbeam::channel::Receiver<ToEditor>,
    opts: ServerOptions,
) -> Result<(), io::Error> {
    // Internal channel
    let (tx, rx) = crossbeam::channel::unbounded();

    let mut editor = Editor::new(runtime, tx, opts);
    editor.on_startup();

    let receivers = [recv, rx];
    let mut recv_select = {
        // Prioritise outside source over internal
        let mut sel = crossbeam::channel::Select::new_biased();
        for r in &receivers {
            sel.recv(r);
        }
        sel
    };

    while editor.is_running {
        use ToEditor::*;

        let Ok(msg) = ({
            let oper = recv_select.select();
            let index = oper.index();
            oper.recv(&receivers[index])
        }) else {
            continue;
        };

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
    }

    Ok(())
}
