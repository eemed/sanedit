mod completion;
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
    completion::draw_completion, prompt::draw_prompt, search::draw_search,
    statusline::draw_statusline, window::draw_window,
};

struct DrawContext<'a, 'b> {
    win: &'a Window,
    buf: &'a Buffer,
    start: &'b mut DrawState,
}

pub(crate) struct DrawState {
    /// Used to track scroll position when drawing prompt
    prompt_scroll_offset: usize,
    compl_scroll_offset: usize,
    // Previously drawn
    prompt: Option<redraw::Prompt>,
    completion: Option<redraw::Completion>,
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
            completion: None,
            statusline: statusline.clone(),
            window: window.clone(),
            msg: None,
        };

        (state, vec![Redraw::Init(window, statusline)])
    }

    pub fn redraw(&mut self, win: &Window, buf: &Buffer, theme: &Theme) -> Vec<Redraw> {
        let mut redraw: Vec<Redraw> = vec![];

        // Close prompt if not focused anymore
        if win.focus() != Focus::Prompt && self.prompt.take().is_some() {
            self.prompt_scroll_offset = 0;
            redraw.push(Redraw::ClosePrompt);
        }

        if win.focus() != Focus::Completion && self.completion.take().is_some() {
            self.compl_scroll_offset = 0;
            redraw.push(Redraw::CloseCompletion);
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

        // Temporary focus
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
            Focus::Completion => {
                let compl = &win.completion;
                let compl = draw_completion(compl, &win.options, &mut self.prompt_scroll_offset);
                match self.completion.as_mut() {
                    Some(prev) => {
                        if let Some(diff) = prev.diff(&compl) {
                            redraw.push(diff.into());
                            *prev = compl;
                        }
                    }
                    None => {
                        redraw.push(Redraw::Completion(compl.clone()));
                        self.completion = Some(compl);
                    }
                }
            }
        }

        redraw
    }
}
