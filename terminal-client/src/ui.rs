use std::io;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Cell, Redraw},
    ClientMessage, Message, Writer,
};

use crate::{grid::Grid, terminal::Terminal};

pub struct UI<W: io::Write> {
    terminal: Terminal,
    grid: Grid,
    writer: Writer<W, Message>,
}

impl<W: io::Write> UI<W> {
    pub fn new(writer: Writer<W, Message>) -> Result<UI<W>> {
        let terminal = Terminal::new()?;
        let width = terminal.width();
        let height = terminal.height();
        Ok(UI {
            terminal,
            grid: Grid::new(width, height),
            writer,
        })
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> bool {
        log::info!("Client got message: {:?}", msg);
        match msg {
            ClientMessage::Hello => {}
            ClientMessage::Redraw(msg) => self.handle_redraw(msg),
            ClientMessage::Flush => self.flush(),
            ClientMessage::Bye => return true,
        }

        false
    }

    fn handle_redraw(&mut self, msg: Redraw) {
        match msg {
            Redraw::Window(win) => {
                log::info!("redraw {}x{}", win.cells().len(), win.cells()[0].len());
                self.grid.push_component(win);
            }
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
}
