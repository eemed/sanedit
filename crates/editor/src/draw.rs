mod completion;
mod filetree;
mod locations;
mod popup;
mod prompt;
mod search;
mod statusline;
mod window;

use std::{
    hash::Hash as _,
    mem,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use sanedit_core::Language;
use sanedit_messages::{
    redraw::{self, window::WindowUpdate, Redraw, Theme},
    ClientMessage,
};
use sanedit_server::{FromEditor, FromEditorSharedMessage};

use crate::editor::{
    buffers::Buffer,
    filetree::Filetree,
    lsp::Lsp,
    windows::{Focus, Window},
    Map,
};

pub(crate) struct EditorContext<'a> {
    pub(crate) win: &'a Window,
    pub(crate) buf: &'a Buffer,
    pub(crate) theme: &'a Theme,
    pub(crate) working_dir: &'a Path,
    pub(crate) filetree: &'a Filetree,
    pub(crate) language_servers: &'a Map<Language, Lsp>,
}

pub(crate) struct DrawContext<'a, 'b> {
    editor: EditorContext<'a>,
    state: &'b mut DrawState,
}

impl<'a, 'b> DrawContext<'a, 'b> {
    pub fn focus_changed_from(&self, focus: Focus) -> bool {
        self.state.last_focus == Some(focus) && focus != self.editor.win.focus()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Hash(u64);

impl Hash {
    pub fn new<H: std::hash::Hash>(typ: &H) -> Hash {
        use std::hash::Hasher;

        let mut hasher = rustc_hash::FxHasher::default();
        typ.hash(&mut hasher);
        Hash(hasher.finish())
    }

    pub fn add<H: std::hash::Hash>(&mut self, typ: &H) {
        use std::hash::Hasher;

        let mut hasher = rustc_hash::FxHasher::default();
        self.0.hash(&mut hasher);
        typ.hash(&mut hasher);
        self.0 = hasher.finish();
    }
}

#[derive(Debug)]
pub(crate) struct DrawState {
    last_focus: Option<Focus>,
    last_prompt: Option<Hash>,
    last_compl: Option<Hash>,
    last_loc: Option<Hash>,
    last_loc_per_group: Map<PathBuf, Hash>,

    last_ft: Option<Hash>,

    last_show_popup: Option<bool>,
    last_statusline: Option<Hash>,

    /// Used to track scroll position when drawing prompt
    prompt_scroll_offset: usize,
    compl_scroll_offset: usize,

    pub(crate) redraw_window: bool,

    pub(crate) window_buffer: Receiver<Arc<FromEditor>>,
    pub(crate) window_buffer_sender: Sender<Arc<FromEditor>>,
    last_window: Option<Hash>,
    last_prompt_selection: Option<usize>,
}

impl DrawState {
    pub fn new(ectx: EditorContext) -> (DrawState, Vec<FromEditorSharedMessage>) {
        let buffer = Arc::new(FromEditor::Message(ClientMessage::Redraw(Redraw::Window(
            WindowUpdate::Full(redraw::window::Window::default()),
        ))));
        let (tx, rx) = channel();
        let _ = tx.send(buffer);

        let mut state = DrawState {
            last_prompt: None,
            last_focus: None,
            last_compl: None,
            last_loc: None,
            last_loc_per_group: Map::default(),
            last_ft: None,
            last_show_popup: None,
            last_statusline: None,
            prompt_scroll_offset: 0,
            compl_scroll_offset: 0,
            redraw_window: true,
            window_buffer: rx,
            window_buffer_sender: tx,
            last_window: None,
            last_prompt_selection: None,
        };

        let mut ctx = DrawContext {
            editor: ectx,
            state: &mut state,
        };

        let mut redraw = vec![];
        if let Some(window) = window::draw(&mut ctx) {
            redraw.push(window);
        }

        if let Some(statusline) = statusline::draw(&mut ctx) {
            redraw.push(statusline);
        }

        (state, redraw)
    }

    pub fn redraw(&mut self, ectx: EditorContext) -> Vec<FromEditorSharedMessage> {
        let EditorContext { win, .. } = ectx;
        let mut redraw: Vec<FromEditorSharedMessage> = vec![];
        let mut ctx = DrawContext {
            editor: ectx,
            state: self,
        };

        if mem::replace(&mut ctx.state.redraw_window, true) {
            if let Some(window) = window::draw(&mut ctx) {
                redraw.push(window);
            }
        }

        if let Some(current) = statusline::draw(&mut ctx) {
            redraw.push(current);
        }

        if let Some(msg) = win.message() {
            let msg: Redraw = msg.clone().into();
            let msg = FromEditorSharedMessage::from(msg);
            redraw.push(msg);
        }

        if let Some(current) = search::draw(&mut ctx) {
            redraw.push(current.into());
        }

        redraw.extend(
            prompt::draw(&mut ctx)
                .into_iter()
                .map(FromEditorSharedMessage::from),
        );

        if let Some(current) = completion::draw(&mut ctx) {
            redraw.push(current.into());
        }

        if let Some(current) = locations::draw(&mut ctx) {
            redraw.push(current.into());
        }

        if let Some(current) = filetree::draw(&mut ctx) {
            redraw.push(current.into());
        }

        if let Some(current) = popup::draw(&mut ctx) {
            redraw.push(current.into());
        }

        self.last_focus = Some(win.focus());
        redraw
    }

    pub fn no_redraw_window(&mut self) {
        self.redraw_window = false;
    }
}
