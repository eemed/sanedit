use std::io::{stdout, BufWriter, Stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor,
    event::*,
    execute, queue, style,
    terminal::{self, *},
};
use sanedit_messages::redraw::{Cell, Color, Style, TextStyle};

pub struct Terminal {
    out: BufWriter<Stdout>,
    written: Vec<Vec<Cell>>,
    brush: Style,
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
            brush: Style::default(),
        })
    }

    pub fn resize(&mut self, width: usize, height: usize) -> Result<()> {
        // Just clear on resize
        self.written = vec![vec![Cell::default(); width]; height];
        write!(self.out, "{}", Clear(ClearType::All))?;
        Ok(())
    }

    pub fn draw_cell(&mut self, cell: &Cell, x: usize, y: usize) -> Result<()> {
        if let Some(written) = self.written.get(y).map(|row| row.get(x)).flatten() {
            if written != cell {
                self.set_style(cell.style)?;
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

    pub fn set_style(&mut self, style: Style) -> Result<()> {
        if self.brush == style {
            return Ok(());
        }
        self.set_text_style(style.text_style)?;
        self.set_color(style.bg, true)?;
        self.set_color(style.fg, false)?;
        self.brush = style;

        Ok(())
    }

    fn set_color(&mut self, color: Option<Color>, is_bg: bool) -> Result<()> {
        if is_bg {
            let bg = color_to_crossterm_color(color.as_ref());
            queue!(self.out, style::SetBackgroundColor(bg))?;
        } else {
            let fg = color_to_crossterm_color(color.as_ref());
            queue!(self.out, style::SetForegroundColor(fg))?;
        }

        Ok(())
    }

    fn set_text_style(&mut self, style: Option<TextStyle>) -> Result<()> {
        if let Some(style) = style {
            let mut attrs = style::Attributes::default();
            if style.contains(TextStyle::BOLD) {
                attrs.set(style::Attribute::Bold);
            }

            if style.contains(TextStyle::UNDERLINE) {
                attrs.set(style::Attribute::Underlined);
            }

            if style.contains(TextStyle::ITALIC) {
                attrs.set(style::Attribute::Italic);
            }
            queue!(self.out, style::SetAttributes(attrs))?;
        } else {
            queue!(self.out, style::SetAttribute(style::Attribute::Reset))?;
        }
        Ok(())
    }

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

pub(crate) fn color_to_crossterm_color(color: Option<&Color>) -> style::Color {
    if let Some(color) = color {
        match color {
            Color::Black => style::Color::Black,
            Color::White => style::Color::White,
            Color::Rgb(rgb) => {
                let (r, g, b) = rgb.get();
                style::Color::Rgb { r, g, b }
            }
        }
    } else {
        style::Color::Reset
    }
}
