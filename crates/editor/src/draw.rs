mod completion;
mod filetree;
mod locations;
mod prompt;
mod search;
mod statusline;
mod window;

use std::{mem, path::Path};

use sanedit_messages::redraw::{Component, Redraw, Theme};

use crate::editor::{
    buffers::Buffer,
    filetree::Filetree,
    windows::{Focus, Window},
};

pub(crate) struct EditorContext<'a> {
    pub(crate) win: &'a Window,
    pub(crate) buf: &'a Buffer,
    pub(crate) theme: &'a Theme,
    pub(crate) working_dir: &'a Path,
    pub(crate) filetree: &'a Filetree,
}

pub(crate) struct DrawContext<'a, 'b> {
    editor: EditorContext<'a>,
    state: &'b mut DrawState,
}

pub(crate) struct DrawState {
    /// Used to detect when prompt is different
    last_prompt: Option<String>,
    last_focus: Option<Focus>,
    last_show_ft: Option<bool>,
    last_show_loc: Option<bool>,

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
        let EditorContext { win, filetree, .. } = ectx;
        let focus_changed_from = |focus| self.last_focus == Some(focus) && focus != win.focus;
        let mut redraw: Vec<Redraw> = vec![];

        let draw = mem::replace(&mut self.redraw, true);
        if !draw {
            return redraw;
        }

        // Send close if not focused
        if focus_changed_from(Focus::Prompt)
            || focus_changed_from(Focus::Search)
            || self
                .last_prompt
                .as_ref()
                .map(|p| p != win.prompt.message())
                .unwrap_or(false)
        {
            self.prompt_scroll_offset = 0;
            self.last_prompt = None;
            redraw.push(Redraw::Prompt(Component::Close));
        }

        if focus_changed_from(Focus::Completion) {
            self.compl_scroll_offset = 0;
            redraw.push(Redraw::Completion(Component::Close));
        }

        let close_ft = !win.ft_view.show && self.last_show_ft == Some(true);
        let unfocus_ft = !close_ft && focus_changed_from(Focus::Filetree);
        let close_loc = !win.locations.show && self.last_show_loc == Some(true);
        let unfocus_loc = !close_loc && focus_changed_from(Focus::Locations);

        if close_ft {
            redraw.push(Redraw::Filetree(Component::Close));
        }

        if close_loc {
            redraw.push(Redraw::Locations(Component::Close));
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

        // Indicate that this is now unfocused
        if unfocus_ft {
            let current = filetree::draw(filetree, &mut ctx);
            redraw.push(current);
        }

        // Indicate that this is now unfocused
        if unfocus_loc {
            let current = locations::draw(&win.locations, &mut ctx);
            redraw.push(current);
        }

        match win.focus() {
            Focus::Search => {
                let current = search::draw(&win.prompt, &win.search, &mut ctx).into();
                redraw.push(current);
            }
            Focus::Prompt => {
                let current = prompt::draw(&win.prompt, &mut ctx).into();
                redraw.push(current);
            }
            Focus::Completion => {
                let current = completion::draw(&win.completion, &mut ctx).into();
                redraw.push(current);
            }
            Focus::Filetree => {
                let current = filetree::draw(filetree, &mut ctx);
                redraw.push(current);
            }
            Focus::Locations => {
                let current = locations::draw(&win.locations, &mut ctx);
                redraw.push(current);
            }
            _ => {}
        }

        self.last_show_ft = Some(win.ft_view.show);
        self.last_show_loc = Some(win.locations.show);
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
