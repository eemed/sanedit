mod context;

use std::io;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Cell, Redraw, Size, Theme},
    ClientMessage, Message, Writer,
};

use crate::{grid::Grid, terminal::Terminal};

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
            grid: Grid::new(width, height),
            context: UIContext::new(width, height),
        })
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> bool {
        // log::info!("Client got message: {:?}", msg);
        match msg {
            ClientMessage::Hello => {}
            ClientMessage::Theme(theme) => self.context.theme = theme,
            ClientMessage::Redraw(msg) => self.handle_redraw(msg),
            ClientMessage::Flush => self.flush(),
            ClientMessage::Bye => return true,
        }

        false
    }

    fn handle_redraw(&mut self, msg: Redraw) {
        match msg {
            Redraw::Window(win) => {
                log::info!("Redraw window");
                self.grid.push_component(win);
            }
            Redraw::Statusline(line) => {
                log::info!("Redraw statusline");
                self.grid.push_component(line);
            }
            Redraw::Prompt(prompt) => {
                log::info!("Redraw prompt");
                self.grid.push_component(prompt);
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

    pub fn size(&self) -> Size {
        Size {
            width: self.terminal.width(),
            height: self.terminal.height(),
        }
    }
}
