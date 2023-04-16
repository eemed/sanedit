mod prompt;
mod search;
mod statusline;
mod window;

use sanedit_messages::redraw::{self, Redraw, Theme};

use crate::editor::{
    buffers::Buffer,
    windows::{Focus, Window},
};

use self::{
    prompt::draw_prompt, search::draw_search, statusline::draw_statusline, window::draw_window,
};

pub(crate) struct DrawState {
    /// Used to track scroll position when drawing prompt
    prompt_scroll_offset: usize,
    // Previously drawn
    prompt: Option<redraw::Prompt>,
    statusline: redraw::Statusline,
    window: redraw::Window,
}

impl DrawState {
    pub fn new(win: &mut Window, buf: &Buffer, theme: &Theme) -> (DrawState, Vec<Redraw>) {
        win.redraw_view(buf);

        let view = win.view();
        let cursors = win.cursors();
        let window = draw_window(view, cursors, buf, theme);
        let statusline = draw_statusline(win, buf);
        let state = DrawState {
            prompt_scroll_offset: 0,
            prompt: None,
            statusline: statusline.clone(),
            window: window.clone(),
        };

        (state, vec![Redraw::Init(window, statusline)])
    }

    pub fn redraw(&mut self, win: &mut Window, buf: &Buffer, theme: &Theme) -> Vec<Redraw> {
        let mut redraw: Vec<Redraw> = vec![];

        if self.prompt.take().is_some() {
            self.prompt_scroll_offset = 0;
            redraw.push(Redraw::ClosePrompt);
        }

        win.redraw_view(buf);

        let view = win.view();
        let cursors = win.cursors();
        let window = draw_window(view, cursors, buf, theme);
        if let Some(diff) = self.window.diff(&window) {
            redraw.push(diff.into());
            self.window = window;
        }

        let statusline = draw_statusline(win, buf);
        if let Some(diff) = self.statusline.diff(&statusline) {
            redraw.push(diff.into());
            self.statusline = statusline;
        }

        match win.focus() {
            Focus::Search => {
                let search = &win.search;
                let search = draw_search(search, &win.options);
                match self.prompt.as_mut() {
                    Some(prev) => {
                        if let Some(diff) = prev.diff(&search) {
                            redraw.push(diff.into());
                            *prev = search;
                        }
                    }
                    None => {
                        redraw.push(Redraw::Prompt(search.clone()));
                        self.prompt = Some(search);
                    }
                }
            }
            Focus::Prompt => {
                let prompt = &win.prompt;
                let prompt = draw_prompt(prompt, &win.options, &mut self.prompt_scroll_offset);
                match self.prompt.as_mut() {
                    Some(prev) => {
                        if let Some(diff) = prev.diff(&prompt) {
                            redraw.push(diff.into());
                            *prev = prompt;
                        }
                    }
                    None => {
                        redraw.push(Redraw::Prompt(prompt.clone()));
                        self.prompt = Some(prompt);
                    }
                }
            }
            Focus::Window => {}
        }

        redraw
    }
}
