mod cell;

use sanedit_buffer::piece_tree::{next_grapheme, PieceTreeSlice};
use sanedit_messages::redraw::{self, Point};

use crate::common::char::{Char, DisplayOptions, GraphemeCategory};
use crate::common::eol::EOL;
use crate::editor::buffers::buffer::Buffer;

pub(crate) use self::cell::Cell;

use super::cursors::{Cursor, Cursors};

#[derive(Debug)]
pub(crate) struct View {
    offset: usize,
    end: usize,
    cells: Vec<Vec<Cell>>,
    primary_cursor: Point,
    width: usize,
    height: usize,
    needs_redraw: bool,
}

impl View {
    pub fn new(width: usize, height: usize) -> View {
        View {
            offset: 0,
            end: 0,
            cells: vec![vec![Cell::default(); width]; height],
            primary_cursor: Point::default(),
            width,
            height,
            needs_redraw: true,
        }
    }

    pub fn clear(&mut self) {
        let width = self.width();
        let height = self.height();
        *self = View::new(width, height);
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Advance line and col in the grid by amount
    fn grid_advance(&mut self, line: &mut usize, col: &mut usize, amount: usize) {
        for _ in 0..amount {
            *col += 1;

            if *col == self.width {
                *line += 1;
                *col = 0;
            }

            if *line == self.height {
                break;
            }
        }
    }

    fn draw_trailing_whitespace(&mut self) {}
    fn draw_end_of_buffer(&mut self) {}

    fn draw_cursors(&mut self, cursors: &Cursors) {
        let primary = cursors.primary();
        self.primary_cursor = self
            .cursor_cell_pos(primary)
            .expect("Primary cursor not in view");
    }

    fn cursor_cell_pos(&mut self, cursor: &Cursor) -> Option<Point> {
        // Cursor is always on a character or at the end of buffer
        let mut last_char: Option<(Point, GraphemeCategory)> = None;

        let mut pos = self.offset;
        for (line, row) in self.cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                if let Some(ch) = cell.char() {
                    if cursor.pos() == pos {
                        return Some(Point { x: col, y: line });
                    }
                    pos += ch.grapheme_len();
                    last_char = Some((Point { x: col, y: line }, ch.grapheme_category()));
                }
            }
        }

        if cursor.pos() == self.end {
            let point = last_char
                .map(|(mut point, category)| {
                    // If we do not have EOL and space available, put cursor to
                    // the right side. Otherwise put cursor to the beginning of the
                    // next line.
                    if point.x + 1 < self.width && category != GraphemeCategory::EOL {
                        point.x += 1;
                        point
                    } else {
                        point.y += 1;
                        point.x = 0;
                        point
                    }
                })
                .unwrap_or(Point::default());
            return Some(point);
        }

        None
    }

    fn draw_cells(&mut self, buf: &Buffer, opts: &DisplayOptions) {
        let slice = buf.slice(self.offset..);
        let mut pos = 0;
        let mut line = 0;
        let mut col = 0;

        while let Some(grapheme) = next_grapheme(&slice, pos) {
            if line == self.height {
                break;
            }
            let grapheme_len = grapheme.len();
            let is_eol = EOL::is_eol(&grapheme);
            let ch = Char::new(grapheme, col, opts);
            let ch_width = ch.width();
            let cell = ch.into();
            self.cells[line][col] = cell;

            self.grid_advance(&mut line, &mut col, ch_width);

            // c_col != 0 because eol maybe on the last cell and we don't
            // want to crate extra empty line
            if is_eol && col != 0 {
                line += 1;
                col = 0;
            }

            pos += grapheme_len;
        }

        self.end = pos;
    }

    pub fn redraw(&mut self, buf: &Buffer, cursors: &Cursors, opts: &DisplayOptions) {
        self.clear();
        self.draw_cells(buf, opts);
        self.draw_cursors(cursors);
        self.draw_end_of_buffer();
        self.draw_trailing_whitespace();
        self.needs_redraw = false;
    }

    pub fn scroll_down(&mut self, buf: &Buffer, cursors: &Cursors, opts: &DisplayOptions) {}

    pub fn scroll_up(&mut self, buf: &Buffer, cursors: &Cursors, opts: &DisplayOptions) {}

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn set_view_offset(&mut self, offset: usize) {
        self.offset = offset;
        self.needs_redraw = true;
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn primary_cursor(&self) -> Point {
        self.primary_cursor
    }

    pub fn last_char_on_line(&self, line: usize) -> Point {
        let mut last = Point { x: 0, y: line };
        let row = &self.cells[line];
        for (col, cell) in row.iter().enumerate() {
            if matches!(cell, Cell::Char { .. }) {
                last = Point { x: col, y: line };
            }
        }

        last
    }

    pub fn pos_at_point(&self, point: Point) -> Option<usize> {
        let mut pos = self.offset;
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Cell::Char { ch } = cell {
                    if point.y == y && point.x == x {
                        return Some(pos);
                    }
                    pos += ch.grapheme_len();
                }
            }
        }

        // TODO handle end of buffer
        // if pos.y == y && c_pos == view.buf_range.end {
        //     return Some(view.buf_range.end);
        // }

        None
    }
}

impl From<&View> for redraw::Window {
    fn from(view: &View) -> Self {
        let mut grid = vec![vec![redraw::Cell::default(); view.width()]; view.height()];

        for (line, row) in view.cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                grid[line][col] = cell.char().map(|ch| ch.display()).unwrap_or(" ").into();
            }
        }

        redraw::Window::new(grid, view.primary_cursor)
    }
}

#[cfg(test)]
mod test {
    use crate::editor::buffers::buffer::Buffer;

    use super::*;

    #[test]
    fn tabs() {
        let width = 80;
        let opts = DisplayOptions::default();
        let mut buf = Buffer::new();
        buf.append("\tHello\tWorld");

        let mut view = View::new(width, 1);
        view.redraw(&buf.slice(..), &Cursors::default(), &opts);

        println!("{}", "-".repeat(width));
        for row in &view.cells {
            for cell in row {
                print!("{}", cell.char().display());
            }
            println!("");
        }
        println!("{}", "-".repeat(width));
    }
}
