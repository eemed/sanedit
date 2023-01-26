use std::io::{stdout, BufWriter, Stdout, Write};

use anyhow::Result;
use crossterm::{cursor, event::*, execute, queue, style, terminal::*};
use sanedit_messages::redraw::Cell;

pub struct Terminal {
    out: BufWriter<Stdout>,
    written: Vec<Vec<Cell>>,
}

impl Terminal {
    pub fn new() -> Result<Terminal> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(ClearType::All),
        )?;
        let (width, height) = terminal_size();

        Ok(Terminal {
            out: BufWriter::with_capacity(4096_000, stdout),
            written: vec![vec![Cell::default(); width]; height],
        })
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.written = vec![vec![Cell::default(); width]; height];
    }

    pub fn draw_cell(&mut self, cell: &Cell, x: usize, y: usize) -> Result<()> {
        if let Some(written) = self.written.get(y).map(|row| row.get(x)).flatten() {
            if cell != "" && written != cell {
                // if cell.style != self.brush {
                //     self.set_style(cell.style)?;
                // }

                self.goto(x, y)?;
                write!(self.out, "{}", cell)?;
                self.written[y][x] = cell.clone();
            }
        }

        Ok(())
    }

    pub fn goto(&mut self, x: usize, y: usize) -> Result<()> {
        queue!(self.out, cursor::MoveTo(x as u16, y as u16))?;
        Ok(())
    }

    // pub fn set_style(&mut self, cell_style: Option<CellStyle>) -> Result<()> {
    //     if let Some(style) = cell_style {
    //         self.set_text_style(style.text_style)?;
    //         queue!(
    //             self.out,
    //             style::SetBackgroundColor(color_to_crossterm_color(style.bg.as_ref()))
    //         )?;
    //         queue!(
    //             self.out,
    //             style::SetForegroundColor(color_to_crossterm_color(style.fg.as_ref()))
    //         )?;
    //     } else {
    //         queue!(self.out, style::ResetColor)?;
    //     }
    //     self.brush = cell_style;

    //     Ok(())
    // }

    // pub fn set_text_style(&mut self, style: Option<TextStyle>) -> Result<()> {
    //     if let Some(style) = style {
    //         let mut attrs = style::Attributes::default();
    //         if style.contains(TextStyle::BOLD) {
    //             attrs.set(style::Attribute::Bold);
    //         }

    //         if style.contains(TextStyle::UNDERLINE) {
    //             attrs.set(style::Attribute::Underlined);
    //         }

    //         if style.contains(TextStyle::ITALIC) {
    //             attrs.set(style::Attribute::Italic);
    //         }
    //         queue!(self.out, style::SetAttributes(attrs))?;
    //     } else {
    //         queue!(self.out, style::SetAttribute(style::Attribute::Reset))?;
    //     }
    //     Ok(())
    // }

    pub fn flush(&mut self) -> Result<()> {
        self.out.flush()?;
        Ok(())
    }

    pub fn height(&self) -> usize {
        self.written.len()
    }

    pub fn width(&self) -> usize {
        self.written.get(0).map(|row| row.len()).unwrap_or(0)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if disable_raw_mode().is_err() {
            log::error!("Failed to disable raw mode");
        }

        if execute!(self.out, DisableMouseCapture, LeaveAlternateScreen).is_err() {
            log::error!("Failed to leave alternate screen");
        }
    }
}

pub(crate) fn terminal_size() -> (usize, usize) {
    size().map_or((80, 24), |(x, y)| (x as usize, y as usize))
}
