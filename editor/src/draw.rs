mod prompt;
mod statusline;
mod window;

use sanedit_messages::redraw::{self, Redraw, Theme};

use crate::editor::{
    buffers::Buffer,
    windows::{Mode, Window},
};

use self::{prompt::draw_prompt, statusline::draw_statusline, window::draw_window};

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
        win.draw_view(buf);

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
        match win.mode() {
            Mode::Normal => {
                if self.prompt.take().is_some() {
                    self.prompt_scroll_offset = 0;
                    redraw.push(Redraw::ClosePrompt);
                }

        log::info!("D state");
                win.draw_view(buf);
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
            }
            Mode::Prompt => {
                let prompt = draw_prompt(&win.prompt, &win.options, &mut self.prompt_scroll_offset);
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
        }

        redraw
    }
}
