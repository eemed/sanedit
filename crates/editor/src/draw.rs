mod completion;
mod prompt;
mod search;
mod statusline;
mod window;

use sanedit_messages::redraw::{self, Component, Diffable, Redraw, Theme};

use crate::editor::{
    buffers::Buffer,
    windows::{Focus, Window},
};

use self::{statusline::draw_statusline, window::draw_window};

pub(crate) struct DrawContext<'a, 'b> {
    win: &'a Window,
    buf: &'a Buffer,
    theme: &'a Theme,
    state: &'b mut DrawState,
}

pub(crate) struct DrawState {
    /// Used to track scroll position when drawing prompt
    prompt_scroll_offset: usize,
    compl_scroll_offset: usize,
    // Previously drawn
    prompt: Option<redraw::Prompt>,
    msg: Option<redraw::StatusMessage>,
    statusline: redraw::Statusline,
    window: redraw::Window,
}

impl DrawState {
    pub fn new(win: &mut Window, buf: &Buffer, theme: &Theme) -> (DrawState, Vec<Redraw>) {
        win.redraw_view(buf);

        let window = draw_window(win, buf, theme);
        let statusline = draw_statusline(win, buf);
        let state = DrawState {
            prompt_scroll_offset: 0,
            compl_scroll_offset: 0,
            prompt: None,
            statusline: statusline.clone(),
            window: window.clone(),
            msg: None,
        };

        (state, vec![Redraw::Init(window, statusline)])
    }

    pub fn redraw(&mut self, win: &Window, buf: &Buffer, theme: &Theme) -> Vec<Redraw> {
        let mut redraw: Vec<Redraw> = vec![];

        if win.focus != Focus::Prompt {
            self.prompt_scroll_offset = 0;
            redraw.push(Redraw::Prompt(Component::Close));
        }

        if win.focus != Focus::Completion {
            self.compl_scroll_offset = 0;
            redraw.push(Redraw::Completion(Component::Close));
        }

        // Window
        let window = draw_window(win, buf, theme);
        if let Some(diff) = self.window.diff(&window) {
            redraw.push(diff.into());
            self.window = window;
        }

        // Statusline
        let statusline = draw_statusline(win, buf);
        if let Some(diff) = self.statusline.diff(&statusline) {
            redraw.push(diff.into());
            self.statusline = statusline;
        }

        // Message
        match (win.message().cloned(), self.msg.clone()) {
            (Some(m), None) => {
                redraw.push(m.clone().into());
                self.msg = Some(m);
            }
            (Some(m1), Some(m2)) => {
                if m1 != m2 {
                    redraw.push(m1.clone().into());
                    self.msg = Some(m1);
                }
            }
            _ => {
                self.msg = None;
            }
        }

        let mut ctx = DrawContext {
            win,
            buf,
            theme,
            state: self,
        };

        match win.focus() {
            Focus::Search => {
                let current = search::draw(&win.search, &mut ctx);
                redraw.push(current);
            }
            Focus::Prompt => {
                let current = prompt::draw(&win.prompt, &mut ctx);
                redraw.push(current);
            }
            Focus::Completion => {
                let current = completion::draw(&win.completion, &mut ctx);
                redraw.push(current);
            }
            _ => {}
        }

        redraw
    }
}
