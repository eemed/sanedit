mod completion;
mod cursors;
mod message;
mod mode;
mod options;
mod prompt;
mod prompt_view;
mod view;

use std::mem;

use sanedit_messages::redraw::{Redraw, Size, Theme};

use crate::editor::buffers::{Buffer, BufferId};

use self::prompt_view::PromptView;
pub(crate) use self::{
    cursors::{Cursor, Cursors},
    message::{Message, Severity},
    mode::Mode,
    options::WindowOptions,
    prompt::Prompt,
    prompt::PromptAction,
    view::View,
};

#[derive(Debug)]
pub(crate) struct Window {
    buf: BufferId,
    view: View,
    message: Option<Message>,
    cursors: Cursors,
    mode: Mode,
    pub prompt: Prompt,
    pub prompt_view: PromptView,
    pub options: WindowOptions,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
            view: View::new(width, height),
            message: None,
            cursors: Cursors::default(),
            prompt: Prompt::default(),
            prompt_view: PromptView::default(),
            options: WindowOptions::default(),
            mode: Mode::Normal,
        }
    }

    pub fn change_buffer(&mut self, bid: BufferId) {
        let width = self.view.width();
        let height = self.view.height();
        *self = Window::new(bid, width, height);
    }

    pub fn info_msg(&mut self, message: String) {
        self.message = Some(Message {
            severity: Severity::Info,
            message,
        });
    }

    pub fn warn_msg(&mut self, message: String) {
        self.message = Some(Message {
            severity: Severity::Warn,
            message,
        });
    }

    pub fn error_msg(&mut self, message: String) {
        self.message = Some(Message {
            severity: Severity::Error,
            message,
        });
    }

    pub fn primary_cursor(&self) -> &Cursor {
        self.cursors.primary()
    }

    pub fn primary_cursor_mut(&mut self) -> &mut Cursor {
        self.view.invalidate();
        self.cursors.primary_mut()
    }

    pub fn scroll_down(&mut self, buf: &Buffer) {
        debug_assert!(buf.id == self.buf, "Provided a wrong buffer to window");
        let mut view = mem::take(&mut self.view);
        view.scroll_down(self, buf);
        self.view = view;

        let primary = self.cursors.primary_mut();
        let range = self.view.range();
        if primary.pos() < range.start {
            primary.goto(range.start);
        }

        log::info!("View range: {range:?}, {}", primary.pos());
    }

    pub fn scroll_up(&mut self, buf: &Buffer) {
        debug_assert!(buf.id == self.buf, "Provided a wrong buffer to window");
        let mut view = mem::take(&mut self.view);
        view.scroll_up(self, buf);
        self.view = view;

        let primary = self.cursors.primary_mut();
        let range = self.view.range();
        if primary.pos() > range.end {
            primary.goto(range.end);
        }

        log::info!("View range: {range:?}, {}", primary.pos());
    }

    /// sets window offset so that primary cursor is visible in the drawn view.
    pub fn view_to_cursor(&mut self, buf: &Buffer) {
        let cursor = self.primary_cursor().pos();
        let mut view = mem::take(&mut self.view);
        view.align_to_show(cursor, self, buf);
        self.view = view;
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buf
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn resize(&mut self, size: Size) {
        self.view.resize(size);
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn open_prompt(&mut self, prompt: Prompt) {
        self.prompt = prompt;
        self.mode = Mode::Prompt;
    }

    pub fn close_prompt(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn cursors(&self) -> &Cursors {
        &self.cursors
    }

    pub fn redraw(&mut self, buf: &Buffer, theme: &Theme) -> Vec<Redraw> {
        let mut redraw = vec![];
        match self.mode {
            Mode::Normal => {
                if let Some(win) = self.redraw_view(buf, theme) {
                    redraw.push(win);
                }

                let statusline = view::draw_statusline(self, buf);
                redraw.push(statusline.into());
            }
            Mode::Prompt => {
                if let Some(prompt) = self.redraw_prompt(theme) {
                    redraw.push(prompt);
                }
            }
        }

        redraw
    }

    fn redraw_prompt(&mut self, theme: &Theme) -> Option<Redraw> {
        let mut prompt = mem::take(&mut self.prompt_view);
        let redraw = prompt.draw_prompt(self, theme).map(|prompt| prompt.into());
        self.prompt_view = prompt;
        redraw
    }

    fn redraw_view(&mut self, buf: &Buffer, theme: &Theme) -> Option<Redraw> {
        let mut view = mem::take(&mut self.view);
        let redraw = view.draw_window(self, buf, theme).map(|view| view.into());
        self.view = view;
        redraw
    }
}
