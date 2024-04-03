mod context;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Point, Size, Style},
    ClientMessage, Message,
};

use crate::{
    grid::{Grid, Rect},
    terminal::Terminal,
};

pub use self::context::UIContext;

pub(crate) struct UI {
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
            context: UIContext::new(),
        })
    }

    pub fn window_rect(&self) -> Rect {
        self.grid.window_area()
    }

    pub fn resize(&mut self, size: Size) {
        self.terminal.resize(size.width, size.height);
        self.grid.resize(size.width, size.height);

        // self.grid.draw(&self.context);
        // self.flush();
    }

    /// Called when client will send input to server
    pub fn on_send_input(&mut self, _msg: &Message) {
        self.grid.on_send_input();
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> bool {
        use ClientMessage::*;
        match msg {
            Hello => {}
            SetOption(_) => {}
            Theme(theme) => {
                self.context.theme = theme;
            }
            Redraw(msg) => {
                self.grid.handle_redraw(&self.context, msg);
            }
            Flush => self.flush(),
            Bye => {
                log::info!("UI got bye, exiting.");
                return true;
            }
        }

        false
    }

    fn flush(&mut self) {
        // log::info!("Flush ui");
        let (cells, cursor) = self.grid.draw(&self.context);
        for (line, row) in cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                self.terminal.draw_cell(cell, col, line);
            }
        }

        if let Some(cursor) = cursor {
            self.terminal.show_cursor();
            let Point { x, y } = cursor.point;
            self.terminal.set_style(Style {
                text_style: None,
                bg: cursor.bg,
                fg: cursor.fg,
            });
            self.terminal.set_cursor_style(cursor.shape);
            self.terminal.goto(x, y);
        } else {
            self.terminal.hide_cursor();
        }

        self.terminal.flush();
    }
}
