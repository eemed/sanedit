mod completion;
mod filetree;
mod prompt;
mod search;
mod statusline;
mod window;

use std::mem;

use sanedit_messages::redraw::{Component, Redraw, Theme};

use crate::editor::{
    buffers::Buffer,
    filetree::Filetree,
    windows::{Focus, Window},
};

pub(crate) struct DrawContext<'a, 'b> {
    win: &'a Window,
    buf: &'a Buffer,
    theme: &'a Theme,
    state: &'b mut DrawState,
}

pub(crate) struct DrawState {
    /// Used to detect when prompt is different
    last_prompt: Option<String>,
    // last_focus
    /// Used to track scroll position when drawing prompt
    prompt_scroll_offset: usize,
    compl_scroll_offset: usize,
    redraw_window: bool,
    redraw: bool,
}

impl DrawState {
    pub fn new(win: &mut Window, buf: &Buffer, theme: &Theme) -> (DrawState, Vec<Redraw>) {
        win.redraw_view(buf);

        let mut state = DrawState {
            last_prompt: None,
            prompt_scroll_offset: 0,
            compl_scroll_offset: 0,
            redraw_window: true,
            redraw: true,
        };

        let mut ctx = DrawContext {
            win,
            buf,
            theme,
            state: &mut state,
        };

        let window = window::draw(&mut ctx).into();
        let statusline = statusline::draw(&mut ctx).into();

        (state, vec![window, statusline])
    }

    pub fn redraw(
        &mut self,
        win: &mut Window,
        buf: &Buffer,
        theme: &Theme,
        filetree: &Filetree,
    ) -> Vec<Redraw> {
        let mut redraw: Vec<Redraw> = vec![];

        let draw = mem::replace(&mut self.redraw, true);
        if !draw {
            return redraw;
        }

        // Send close if not focused
        if win.focus != Focus::Prompt
            || self
                .last_prompt
                .as_ref()
                .map(|p| p == win.prompt.message())
                .unwrap_or(false)
        {
            self.prompt_scroll_offset = 0;
            self.last_prompt = None;
            redraw.push(Redraw::Prompt(Component::Close));
        }

        if win.focus != Focus::Completion {
            self.compl_scroll_offset = 0;
            redraw.push(Redraw::Completion(Component::Close));
        }

        if win.focus != Focus::Filetree {
            redraw.push(Redraw::Filetree(Component::Close));
        }

        let draw_win = mem::replace(&mut self.redraw_window, true);
        let draw_lnr = draw_win && win.options.show_linenumbers;
        if draw_win {
            win.redraw_view(buf);
        }

        let mut ctx = DrawContext {
            win,
            buf,
            theme,
            state: self,
        };

        if draw_win {
            let window = window::draw(&mut ctx);
            if draw_lnr {
                // let lnrs = window::draw_line_numbers(&ctx);
                // redraw.push(lnrs.into());
                redraw.push(window.into());
            } else {
                redraw.push(window.into());
            }
        }

        let statusline = statusline::draw(&mut ctx).into();
        redraw.push(statusline);

        if let Some(msg) = win.message() {
            redraw.push(msg.clone().into());
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
            _ => {}
        }

        redraw
    }

    pub fn no_redraw(&mut self) {
        self.redraw = false;
    }

    pub fn no_redraw_window(&mut self) {
        self.redraw_window = false;
    }
}
