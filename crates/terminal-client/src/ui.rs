mod context;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Point, Size, Style},
    ClientMessage, Message,
};

use crate::{
    grid::{Drawable, Grid},
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
            grid: Grid::new(width, height),
            context: UIContext::new(width, height),
        })
    }

    pub fn window_position(&self) -> Point {
        self.grid.window.position(&self.context)
    }

    pub fn window_size(&self) -> Size {
        self.grid.window.size(&self.context)
    }

    pub fn resize(&mut self, size: Size) {
        self.context.width = size.width;
        self.context.height = size.height;
        self.terminal.resize(size.width, size.height);
        self.grid.resize(size.width, size.height);
    }

    /// Called when client will send input to server
    pub fn on_send_input(&mut self, _msg: &Message) {
        self.grid.msg = None;
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> bool {
        // log::info!("Client got message: {:?}", msg);
        match msg {
            ClientMessage::Hello => {}
            ClientMessage::Theme(theme) => self.context.theme = theme,
            ClientMessage::Redraw(msg) => self.grid.handle_redraw(msg),
            ClientMessage::Flush => self.flush(),
            ClientMessage::Bye => {
                log::info!("UI got bye, exiting.");
                return true;
            }
        }

        false
    }

    fn flush(&mut self) {
        let (cells, cursor) = self.grid.draw(&self.context);
        for (line, row) in cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                self.terminal.draw_cell(cell, col, line);
            }
        }

        let Point { x, y } = cursor.point;
        self.terminal.set_style(Style {
            text_style: None,
            bg: cursor.bg,
            fg: cursor.fg,
        });
        self.terminal.set_cursor_style(cursor.shape);
        self.terminal.goto(x, y);
        self.terminal.flush();
    }
}
