mod context;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Point, Size, Style},
    ClientMessage, Message,
};

use crate::{
    grid::{Grid, Rect, RedrawResult},
    terminal::Terminal,
};

pub(crate) use self::context::UIContext;

pub(crate) struct UI {
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

    pub fn window(&self) -> Rect {
        // Atleast size of 1 cell
        let mut rect = self.grid.window();
        rect.width = rect.width.max(1);
        rect.height = rect.height.max(1);
        rect
    }

    pub fn resize(&mut self, size: Size) -> anyhow::Result<()> {
        self.terminal.resize(size.width, size.height)?;
        self.grid.resize(size.width, size.height);

        // self.grid.draw(&self.context);
        // self.flush();
        Ok(())
    }

    /// Called when client will send input to server
    pub fn on_send_input(&mut self, msg: &Message) {
        self.grid.on_send_input(msg);
    }

    /// Called when client will send input to server
    pub fn on_focus_change(&mut self, focus: bool) {
        self.grid.on_focus_change(focus);
        let _ = self.draw_all();
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> anyhow::Result<UIResult> {
        use ClientMessage::*;
        match msg {
            Hello => {}
            Theme(theme) => {
                self.grid.theme = theme.into();
            }
            Redraw(msg) => match self.grid.handle_redraw(msg) {
                RedrawResult::Resized => return Ok(UIResult::Resize),
                RedrawResult::Ok => {}
            },
            Flush => self.draw_all()?,
            Bye => {
                log::info!("UI got bye, exiting.");
                return Ok(UIResult::Exit);
            }
        }

        Ok(UIResult::Nothing)
    }

    fn draw_all(&mut self) -> anyhow::Result<()> {
        // log::info!("Flush ui");
        let (cells, cursor) = self.grid.draw();
        for (line, row) in cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                self.terminal.draw_cell(cell, col, line)?;
            }
        }

        if let Some(cursor) = cursor {
            self.terminal.show_cursor()?;
            let Point { x, y } = cursor.point;
            self.terminal.set_style(Style {
                text_style: None,
                bg: cursor.bg,
                fg: cursor.fg,
            })?;
            self.terminal.set_cursor_style(cursor.shape)?;
            self.terminal.goto(x, y)?;
        } else {
            self.terminal.hide_cursor()?;
        }

        self.terminal.flush()?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum UIResult {
    Nothing,
    Exit,
    Resize,
}
