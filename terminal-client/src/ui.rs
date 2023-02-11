use std::io;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Cell, Redraw, Size},
    ClientMessage, Message, Writer,
};

use crate::{grid::Grid, terminal::Terminal};

pub struct UI {
    terminal: Terminal,
    grid: Grid,
}

impl UI {
    pub fn new() -> Result<UI> {
        let terminal = Terminal::new()?;
        let width = terminal.width();
        let height = terminal.height();
        Ok(UI {
            terminal,
            grid: Grid::new(width, height),
        })
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> bool {
        // log::info!("Client got message: {:?}", msg);
        match msg {
            ClientMessage::Hello => {}
            ClientMessage::Redraw(msg) => self.handle_redraw(msg),
            ClientMessage::Flush => self.flush(),
            ClientMessage::Bye => return true,
            ClientMessage::Theme(_) => todo!(),
        }

        false
    }

    fn handle_redraw(&mut self, msg: Redraw) {
        match msg {
            Redraw::Window(win) => {
                self.grid.push_component(win);
            }
            Redraw::Statusline(line) => self.grid.push_component(line),
            Redraw::Prompt(prompt) => self.grid.push_component(prompt),
        }
    }

    fn flush(&mut self) {
        let (cells, cursor) = self.grid.draw();
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
