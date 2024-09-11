mod completion;
mod filetree;
mod locations;
mod popup;
mod prompt;
mod search;
mod statusline;
mod window;

use std::{mem, path::Path};

use sanedit_core::Filetype;
use sanedit_messages::redraw::{Redraw, Theme};

use crate::{
    actions::jobs::LSPHandle,
    editor::{
        buffers::Buffer,
        filetree::Filetree,
        windows::{Focus, Window},
        Map,
    },
};

pub(crate) struct EditorContext<'a> {
    pub(crate) win: &'a Window,
    pub(crate) buf: &'a Buffer,
    pub(crate) theme: &'a Theme,
    pub(crate) working_dir: &'a Path,
    pub(crate) filetree: &'a Filetree,
    pub(crate) language_servers: &'a Map<Filetype, LSPHandle>,
}

pub(crate) struct DrawContext<'a, 'b> {
    editor: EditorContext<'a>,
    state: &'b mut DrawState,
}

impl<'a, 'b> DrawContext<'a, 'b> {
    pub fn focus_changed_from(&self, focus: Focus) -> bool {
        self.state.last_focus == Some(focus) && focus != self.editor.win.focus
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
    redraw: bool,

    pub(crate) redraw_window: bool,
}

impl DrawState {
    pub fn new(ectx: EditorContext) -> (DrawState, Vec<Redraw>) {
        let mut state = DrawState {
            last_prompt: None,
            last_focus: None,
            last_show_ft: None,
            last_show_loc: None,
            last_show_popup: None,
            prompt_scroll_offset: 0,
            compl_scroll_offset: 0,
            redraw_window: true,
            redraw: true,
        };

        let mut ctx = DrawContext {
            editor: ectx,
            state: &mut state,
        };

        let window = window::draw(&mut ctx).into();
        let statusline = statusline::draw(&mut ctx).into();

        (state, vec![window, statusline])
    }

    pub fn redraw(&mut self, ectx: EditorContext) -> Vec<Redraw> {
        let EditorContext { win, .. } = ectx;
        let mut redraw: Vec<Redraw> = vec![];

        let draw = mem::replace(&mut self.redraw, true);
        if !draw {
            return redraw;
        }

        let mut ctx = DrawContext {
            editor: ectx,
            state: self,
        };

        if mem::replace(&mut ctx.state.redraw_window, true) {
            let window = window::draw(&mut ctx);
            redraw.push(window.into());
        }

        let statusline = statusline::draw(&mut ctx).into();
        redraw.push(statusline);

        if let Some(msg) = win.message() {
            redraw.push(msg.clone().into());
        }

        if let Some(current) = search::draw(&mut ctx) {
            redraw.push(current);
        }

        redraw.extend(prompt::draw(&mut ctx));

        if let Some(current) = completion::draw(&mut ctx) {
            redraw.push(current);
        }

        if let Some(current) = locations::draw(&mut ctx) {
            redraw.push(current);
        }

        if let Some(current) = filetree::draw(&mut ctx) {
            redraw.push(current);
        }

        if let Some(current) = popup::draw(&mut ctx) {
            redraw.push(current);
        }

        self.last_focus = Some(win.focus);
        redraw
    }

    pub fn no_redraw(&mut self) {
        self.redraw = false;
    }

    pub fn no_redraw_window(&mut self) {
        self.redraw_window = false;
    }
}
