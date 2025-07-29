mod completion;
mod filetree;
mod locations;
mod popup;
mod prompt;
mod search;
mod statusline;
mod window;

use std::{
    mem,
    path::Path,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use sanedit_core::Language;
use sanedit_messages::{
    redraw::{self, Component, Redraw, Theme},
    ClientMessage,
};
use sanedit_server::{FromEditor, FromEditorSharedMessage};

use crate::editor::{
    buffers::Buffer,
    filetree::Filetree,
    lsp::LSP,
    windows::{Focus, Window},
    Map,
};

pub(crate) struct EditorContext<'a> {
    pub(crate) win: &'a Window,
    pub(crate) buf: &'a Buffer,
    pub(crate) theme: &'a Theme,
    pub(crate) working_dir: &'a Path,
    pub(crate) filetree: &'a Filetree,
    pub(crate) language_servers: &'a Map<Language, LSP>,
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

#[derive(Debug)]
pub(crate) struct DrawState {
    /// Used to detect when prompt is different
    last_prompt: Option<String>,
    last_focus: Option<Focus>,
    last_show_ft: Option<bool>,
    last_show_loc: Option<bool>,
    last_show_popup: Option<bool>,

    /// Used to track scroll position when drawing prompt
    prompt_scroll_offset: usize,
    compl_scroll_offset: usize,

    pub(crate) redraw_window: bool,

    pub(crate) window_buffer: Receiver<Arc<FromEditor>>,
    pub(crate) window_buffer_sender: Sender<Arc<FromEditor>>,
}

impl DrawState {
    pub fn new(ectx: EditorContext) -> (DrawState, Vec<FromEditorSharedMessage>) {
        let buffer = Arc::new(FromEditor::Message(ClientMessage::Redraw(Redraw::Window(
            Component::Open(redraw::window::Window::default()),
        ))));
        let (tx, rx) = channel();
        let _ = tx.send(buffer);

        let mut state = DrawState {
            last_prompt: None,
            last_focus: None,
            last_show_ft: None,
            last_show_loc: None,
            last_show_popup: None,
            prompt_scroll_offset: 0,
            compl_scroll_offset: 0,
            redraw_window: true,
            window_buffer: rx,
            window_buffer_sender: tx,
        };

        let mut ctx = DrawContext {
            editor: ectx,
            state: &mut state,
        };

        let mut redraw = vec![];
        if let Some(window) = window::draw(&mut ctx) {
            redraw.push(window);
        }
        let statusline: Redraw = statusline::draw(&mut ctx).into();
        let statusline = FromEditorSharedMessage::from(statusline);
        redraw.push(statusline);

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

        let statusline: Redraw = statusline::draw(&mut ctx).into();
        let statusline = FromEditorSharedMessage::from(statusline);
        redraw.push(statusline);

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
