use std::io;

use anyhow::Result;
use sanedit_messages::{ClientMessage, Message, Redraw, Writer};

use crate::{
    grid::{Cell, Grid},
    terminal::Terminal,
};

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
            Redraw::Window(redraw) => {
                log::info!("redraw {}x{}", redraw.len(), redraw[0].len());
                let mut cells =
                    vec![vec![Cell::default(); self.terminal.width()]; self.terminal.height()];
                for (line, row) in redraw.iter().enumerate() {
                    for (col, content) in row.iter().enumerate() {
                        cells[line][col] = Cell::from(content.as_str());
                    }
                }
                self.grid.push_component(cells);
            }
        }
    }

    fn flush(&mut self) {
        let cells = self.grid.draw();
        for (line, row) in cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                self.terminal.draw_cell(cell, col, line);
            }
        }

        self.terminal.flush();
    }
}
