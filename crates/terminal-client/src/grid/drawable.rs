use sanedit_messages::redraw::{
    statusline::Statusline, Cell, Cursor, Severity, Size, StatusMessage, Style, ThemeField,
};
use unicode_width::UnicodeWidthChar as _;

use crate::ui::UIContext;

use super::{border::Border, Rect};

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
        debug_assert!(
            y < self.rect.height,
            "Invalid y: {y}, height: {}",
            self.rect.height
        );
        debug_assert!(
            x < self.rect.width,
            "Invalid x: {x}, width: {}",
            self.rect.width
        );
        &mut self.cells[self.rect.y + y][self.rect.x + x]
    }

    pub fn replace(&mut self, y: usize, x: usize, cell: Cell) {
        debug_assert!(
            y < self.rect.height,
            "Invalid y: {y}, height: {}",
            self.rect.height
        );
        debug_assert!(
            x < self.rect.width,
            "Invalid x: {x}, width: {}",
            self.rect.width
        );
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
        if self.width() <= 2 || self.height() <= 2 {
            return *self.rect;
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

        let mut result = *self.rect;
        result.x += 1;
        result.y += 1;
        result.width = result.width.saturating_sub(2);
        result.height = result.height.saturating_sub(2);
        result
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    pub fn put_ch(&mut self, row: usize, mut col: usize, ch: char, style: Style) -> usize {
        let ocol = col;
        if col >= self.rect.width {
            return 0;
        }
        let width = ch.width().unwrap_or(1).max(1);

        self.replace(row, col, Cell::new_char(ch, style));
        col += 1;

        while col < ocol + width {
            self.replace(row, col, Cell::padding(style));
            col += 1;
        }

        col - ocol
    }

    pub fn style_line(&mut self, row: usize, style: Style) {
        let mut x = 0;
        while x < self.width() {
            self.cells[self.rect.y + row][self.rect.x + x].style = style;
            x += 1;
        }
    }

    /// Will put a string on a line will and will cut short if it would go over the column
    /// Returns the added amount
    pub fn put_string(&mut self, row: usize, mut col: usize, string: &str, style: Style) -> usize {
        let ocol = col;
        for ch in string
            .chars()
            .map(|ch| if ch.is_control() { ' ' } else { ch })
        {
            if col >= self.rect.width {
                break;
            }
            let width = ch.width().unwrap_or(1).max(1);
            let target = col + width;

            self.replace(row, col, Cell::new_char(ch, style));
            col += 1;

            while col < target {
                self.replace(row, col, Cell::padding(style));
                col += 1;
            }
        }
        col - ocol
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

        let mut result = *self.rect;
        result.width -= 1;
        result
    }
}

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: Subgrid);
    fn cursor(&self, ctx: &UIContext) -> DrawCursor;
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
        let mut x = 0;
        x += grid.put_string(0, x, &self.left, style);

        while x < width {
            grid.replace(0, x, Cell::with_style(style));
            x += 1;
        }

        for (i, cell) in self
            .right
            .chars()
            .map(|ch| if ch.is_control() { ' ' } else { ch })
            .map(|ch| Cell::new_char(ch, style))
            .rev()
            .enumerate()
        {
            let pos = width.saturating_sub(1 + i);
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
        let mut x = 0;
        x += grid.put_string(0, x, &self.message, style);

        while x < width {
            grid.replace(0, x, Cell::with_style(style));
            x += 1;
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        DrawCursor::Ignore
    }
}
