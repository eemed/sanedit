use std::cmp::min;

use sanedit_messages::redraw::{
    statusline::Statusline, window::Window, Cell, Cursor, Severity, Size, StatusMessage, Style,
    ThemeField,
};

use crate::ui::UIContext;

use super::{
    border::Border,
    cell_format::{into_cells_with_style, into_cells_with_theme_pad_with},
    Rect,
};

#[derive(Debug)]
pub enum DrawCursor {
    /// Hide cursor from view
    Hide,
    /// Show cursor
    Show(Cursor),
    /// Keep where ever the cursor is now
    Ignore,
}

pub(crate) struct Subgrid<'a, 'b> {
    pub(super) cells: &'a mut Vec<Vec<Cell>>,
    pub(super) rect: &'b Rect,
}

impl<'a, 'b> Subgrid<'a, 'b> {
    pub fn width(&self) -> usize {
        self.rect.width
    }

    pub fn height(&self) -> usize {
        self.rect.height
    }

    pub fn at(&mut self, y: usize, x: usize) -> &mut Cell {
        debug_assert!(y < self.rect.height, "Invalid y: {y}, height: {}", self.rect.height);
        debug_assert!(x < self.rect.width, "Invalid x: {x}, width: {}", self.rect.width);
        &mut self.cells[self.rect.y + y][self.rect.x + x]
    }

    pub fn replace(&mut self, y: usize, x: usize, cell: Cell) {
        debug_assert!(y < self.rect.height, "Invalid y: {y}, height: {}", self.rect.height);
        debug_assert!(x < self.rect.width, "Invalid x: {x}, width: {}", self.rect.width);
        self.cells[self.rect.y + y][self.rect.x + x] = cell;
    }

    pub fn clear_all(&mut self, style: Style) {
        for y in 0..self.rect.height {
            for x in 0..self.rect.width {
                self.replace(y, x, Cell::with_style(style));
            }
        }
    }

    pub fn draw_border(&mut self, border: Border, style: Style) -> Rect {
        if self.width() <= 2 && self.height() <= 2 {
            return self.rect.clone();
        }

        // Top and bottom
        for x in self.rect.x..self.rect.x + self.rect.width {
            self.cells[self.rect.y][x] = Cell {
                text: border.top().into(),
                style,
            };

            self.cells[self.rect.y + self.rect.height - 1][x] = Cell {
                text: border.bottom().into(),
                style,
            }
        }

        // Sides
        for y in self.rect.y..self.rect.y + self.rect.height {
            self.cells[y][self.rect.x] = Cell {
                text: border.left().into(),
                style,
            };
            self.cells[y][self.rect.x + self.rect.width - 1] = Cell {
                text: border.right().into(),
                style,
            };
        }

        // corners
        self.cells[self.rect.y][self.rect.x] = Cell {
            text: border.top_left().into(),
            style,
        };

        self.cells[self.rect.y + self.rect.height - 1][self.rect.x] = Cell {
            text: border.bottom_left().into(),
            style,
        };

        self.cells[self.rect.y][self.rect.x + self.rect.width - 1] = Cell {
            text: border.top_right().into(),
            style,
        };

        self.cells[self.rect.y + self.rect.height - 1][self.rect.x + self.rect.width - 1] = Cell {
            text: border.bottom_right().into(),
            style,
        };

        let mut result = self.rect.clone();
        result.x += 1;
        result.y += 1;
        result.width -= 2;
        result.height -= 2;
        result
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    pub fn put_line(&mut self, row: usize, line: Vec<Cell>) {
        let y = self.rect.y + row;
        for (mut x, cell) in line.into_iter().enumerate() {
            x += self.rect.x;
            self.cells[y][x] = cell;
        }
    }

    pub fn rect(&self) -> &Rect {
        self.rect
    }

    pub fn subgrid<'c>(self, rect: &'c Rect) -> Subgrid<'a, 'c> {
        Subgrid {
            cells: self.cells,
            rect,
        }
    }

    pub fn draw_separator_right(&mut self, cell: Cell) -> Rect {
        for y in self.rect.y..self.rect.y + self.rect.height {
            self.cells[y][self.rect.x + self.rect.width - 1] = cell.clone();
        }

        let mut result = self.rect.clone();
        result.width -= 1;
        result
    }
}

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: Subgrid);
    fn cursor(&self, ctx: &UIContext) -> DrawCursor;
}

impl Drawable for Window {
    fn draw(&self, _ctx: &UIContext, mut grid: Subgrid) {
        let width = min(grid.width(), self.cells.width());
        let height = min(grid.height(), self.cells.height());

        for x in 0..width {
            for y in 0..height {
                grid.replace(y, x, self.cells[y][x].clone().into());
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
    fn draw(&self, ctx: &UIContext, mut grid: Subgrid) {
        let field = if ctx.client_in_focus {
            ThemeField::Statusline
        } else {
            ThemeField::StatuslineNoFocus
        };
        let style = ctx.style(field);
        let width = grid.width();
        let left = into_cells_with_theme_pad_with(&self.left, &style, width);
        for (i, cell) in left.into_iter().enumerate() {
            grid.replace(0, i, cell);
        }

        let right = into_cells_with_style(&self.right, style);
        for (i, cell) in right.into_iter().rev().enumerate() {
            let pos = width - 1 - i;
            let c = grid.at(0, pos);
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
    fn draw(&self, ctx: &UIContext, mut grid: Subgrid) {
        let field = match self.severity {
            Severity::Hint => ThemeField::Hint,
            Severity::Info => ThemeField::Info,
            Severity::Warn => ThemeField::Warn,
            Severity::Error => ThemeField::Error,
        };
        let style = ctx.style(field);
        let width = grid.width();
        for (i, cell) in into_cells_with_theme_pad_with(&self.message, &style, width)
            .into_iter()
            .enumerate()
        {
            grid.replace(0, i, cell);
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}
