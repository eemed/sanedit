use std::io::{stdout, BufWriter, Stdout, Write};

use anyhow::Result;
use crossterm::{cursor, event::*, execute, queue, style, terminal::*};
use sanedit_messages::redraw::{
    text_style::{self, TextStyle},
    Cell, Color, CursorShape, Style,
};

pub struct Terminal {
    out: BufWriter<Stdout>,
    written: Vec<Vec<Cell>>,
    brush: Style,
    cursor_shown: bool,
}

impl Terminal {
    pub fn new() -> Result<Terminal> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange,
            Clear(ClearType::All),
        )?;
        let (width, height) = terminal_size();

        Ok(Terminal {
            out: BufWriter::with_capacity(4_096_000, stdout),
            written: vec![vec![Cell::default(); width]; height],
            brush: Style::default(),
            cursor_shown: false,
        })
    }

    pub fn resize(&mut self, width: usize, height: usize) -> Result<()> {
        // Just clear on resize
        self.written = vec![vec![Cell::default(); width]; height];
        write!(self.out, "{}", Clear(ClearType::All))?;
        Ok(())
    }

    pub fn draw_cell(&mut self, cell: &Cell, x: usize, y: usize) -> Result<()> {
        if let Some(written) = self.written.get(y).and_then(|row| row.get(x)) {
            if written != cell {
                self.set_style(cell.style)?;
                self.goto(x, y)?;
                write!(self.out, "{}", cell)?;
                self.written[y][x] = cell.clone();
            }
        }

        Ok(())
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        if !self.cursor_shown {
            queue!(self.out, cursor::Show)?;
            self.cursor_shown = true;
        }
        Ok(())
    }

    pub fn hide_cursor(&mut self) -> Result<()> {
        if self.cursor_shown {
            queue!(self.out, cursor::Hide)?;
            self.cursor_shown = false;
        }
        Ok(())
    }

    pub fn set_cursor_style(&mut self, style: CursorShape) -> Result<()> {
        if style.blink() {
            queue!(self.out, cursor::EnableBlinking)?;
        } else {
            queue!(self.out, cursor::DisableBlinking)?;
        }

        let cstyle = match style {
            CursorShape::Block(true) => cursor::SetCursorStyle::BlinkingBlock,
            CursorShape::Block(false) => cursor::SetCursorStyle::SteadyBlock,
            CursorShape::Underline(true) => cursor::SetCursorStyle::BlinkingUnderScore,
            CursorShape::Underline(false) => cursor::SetCursorStyle::SteadyUnderScore,
            CursorShape::Line(true) => cursor::SetCursorStyle::BlinkingBar,
            CursorShape::Line(false) => cursor::SetCursorStyle::SteadyBar,
        };

        queue!(self.out, cstyle)?;
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
        queue!(self.out, style::SetAttribute(style::Attribute::Reset))?;

        if let Some(style) = style {
            let mut attrs = style::Attributes::default();

            if style & text_style::BOLD != 0 {
                attrs.set(style::Attribute::Bold);
            }

            if style & text_style::UNDERLINE != 0 {
                attrs.set(style::Attribute::Underlined);
            }

            if style & text_style::ITALIC != 0 {
                attrs.set(style::Attribute::Italic);
            }

            queue!(self.out, style::SetAttributes(attrs))?;
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
        self.written.first().map(|row| row.len()).unwrap_or(0)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if disable_raw_mode().is_err() {
            log::error!("Failed to disable raw mode");
        }

        if execute!(
            self.out,
            DisableMouseCapture,
            LeaveAlternateScreen,
            DisableFocusChange,
            cursor::Show,
            cursor::SetCursorStyle::DefaultUserShape
        )
        .is_err()
        {
            log::error!("Failed to restore screen");
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
