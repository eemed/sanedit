use std::cmp::min;

use sanedit_messages::redraw::{
    statusline::Statusline, window::Window, Cursor, Severity, StatusMessage, ThemeField,
};

use crate::ui::UIContext;

use super::ccell::{into_cells_with_style, into_cells_with_theme_pad_with, CCell};

#[derive(Debug)]
pub enum DrawCursor {
    /// Hide cursor from view
    Hide,
    /// Show cursor
    Show(Cursor),
    /// Keep where ever the cursor is now
    Ignore,
}

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]);
    fn cursor(&self, ctx: &UIContext) -> DrawCursor;
}

impl Drawable for Window {
    fn draw(&self, _ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let width = min(
            cells.first().map(|c| c.len()).unwrap_or(0),
            self.cells.first().map(|c| c.len()).unwrap_or(0),
        );
        let height = min(cells.len(), self.cells.len());

        for x in 0..width {
            for y in 0..height {
                cells[y][x] = self.cells[y][x].clone().into();
            }
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        match self.cursor {
            Some(cursor) => DrawCursor::Show(cursor),
            None => DrawCursor::Ignore,
        }
    }
}

impl Drawable for Statusline {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let field = if self.client_in_focus { ThemeField::Statusline } else { ThemeField::StatuslineNoFocus };
        let style = ctx.style(field);
        let width = cells.first().map(|c| c.len()).unwrap_or(0);
        let left = into_cells_with_theme_pad_with(&self.left, &style, width);
        for (i, cell) in left.into_iter().enumerate() {
            cells[0][i] = cell;
        }

        let right = into_cells_with_style(&self.right, style);
        for (i, cell) in right.into_iter().rev().enumerate() {
            let pos = width - 1 - i;
            let c = &mut cells[0][pos];
            if c.is_blank() {
                *c = cell;
            } else {
                break;
            }
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}

impl Drawable for StatusMessage {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let field = match self.severity {
            Severity::Hint => ThemeField::Hint,
            Severity::Info => ThemeField::Info,
            Severity::Warn => ThemeField::Warn,
            Severity::Error => ThemeField::Error,
        };
        let style = ctx.style(field);
        let width = cells.first().map(|c| c.len()).unwrap_or(0);
        for (i, cell) in into_cells_with_theme_pad_with(&self.message, &style, width)
            .into_iter()
            .enumerate()
        {
            cells[0][i] = cell;
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}
