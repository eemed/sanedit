mod context;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Redraw, Size},
    ClientMessage,
};

use crate::{
    grid::{Component, Grid},
    terminal::Terminal,
};

pub use self::context::UIContext;

pub struct UI {
    terminal: Terminal,
    grid: Grid,
    context: UIContext,
}

impl UI {
    pub fn new() -> Result<UI> {
        let terminal = Terminal::new()?;
        let width = terminal.width();
        let height = terminal.height();
        Ok(UI {
            terminal,
            grid: Grid::new(),
            context: UIContext::new(width, height),
        })
    }

    pub fn window_size(&self) -> Size {
        self.grid.window.size(&self.context)
    }

    pub fn resize(&mut self, size: Size) {
        self.context.width = size.width;
        self.context.height = size.height;
        self.terminal.resize(size.width, size.height);
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> bool {
        // log::info!("Client got message: {:?}", msg);
        match msg {
            ClientMessage::Hello => {}
            ClientMessage::Theme(theme) => self.context.theme = theme,
            ClientMessage::Redraw(msg) => self.handle_redraw(msg),
            ClientMessage::Flush => self.flush(),
            ClientMessage::Bye => {
                log::info!("UI got bye, exiting.");
                return true;
            }
        }

        false
    }

    fn handle_redraw(&mut self, msg: Redraw) {
        match msg {
            Redraw::Init(win, statusline) => {
                self.grid.window = win;
                self.grid.statusline = statusline;
            }
            Redraw::WindowUpdate(diff) => {
                self.grid.window.update(diff);
            }
            Redraw::StatuslineUpdate(diff) => {
                self.grid.statusline.update(diff);
            }
            Redraw::Prompt(prompt) => {
                self.grid.prompt = Some(prompt);
            }
            Redraw::PromptUpdate(diff) => {
                if let Some(ref mut prompt) = self.grid.prompt {
                    prompt.update(diff);
                }
            }
            Redraw::ClosePrompt => {
                self.grid.prompt = None;
            }
        }
    }

    fn flush(&mut self) {
        let (cells, cursor) = self.grid.draw(&self.context);
        for (line, row) in cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                self.terminal.draw_cell(cell, col, line);
            }
        }

        self.terminal.goto(cursor.x, cursor.y);
        self.terminal.flush();
    }
}
