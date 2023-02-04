mod cell;

use sanedit_buffer::piece_tree::{next_grapheme, PieceTreeSlice};
use sanedit_messages::redraw::{self, Point, Size};

use crate::common::char::{Char, DisplayOptions, GraphemeCategory, Replacement};
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

    /// Display options which were used to draw this view
    display_options: DisplayOptions,
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
            display_options: DisplayOptions::default(),
        }
    }

    pub fn clear(&mut self) {
        self.cells = vec![vec![Cell::default(); self.width]; self.height];
        self.needs_redraw = true;
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

    fn draw_cells(&mut self, buf: &Buffer) {
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
            let ch = Char::new(grapheme, col, &self.display_options);
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

        if pos == buf.len() {
            self.cells[line][col] = Cell::EOF;
        }

        self.end = pos;
    }

    pub fn redraw(&mut self, buf: &Buffer, cursors: &Cursors, opts: &DisplayOptions) {
        self.clear();
        self.display_options = opts.clone();
        self.draw_cells(buf);
        self.draw_cursors(cursors);
        self.needs_redraw = false;
    }

    pub fn scroll_down(&mut self, buf: &Buffer, cursors: &Cursors) {}

    pub fn scroll_up(&mut self, buf: &Buffer, cursors: &Cursors) {}

    pub fn offset(&self) -> usize {
        self.offset
    }

    /// wether this view includes the buffer start
    pub fn at_start(&self) -> bool {
        self.offset == 0
    }

    /// wether this view includes the buffer end
    pub fn at_end(&self) -> bool {
        self.cells[self.height - 1]
            .iter()
            .fold(true, |acc, cell| acc && matches!(cell, Cell::Empty))
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

    pub fn last_non_empty_cell(&self, line: usize) -> Option<Point> {
        let mut last = None;
        let row = &self.cells[line];
        for (col, cell) in row.iter().enumerate() {
            if !matches!(cell, Cell::Empty) {
                last = Some(Point { x: col, y: line });
            }
        }

        last
    }

    pub fn pos_at_point(&self, point: Point) -> Option<usize> {
        let mut pos = self.offset;
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                match cell {
                    Cell::EOF => {
                        if point.y == y && point.x == x {
                            return Some(pos);
                        }
                    }
                    Cell::Char { ch } => {
                        if point.y == y && point.x == x {
                            return Some(pos);
                        }
                        pos += ch.grapheme_len();
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub fn resize(&mut self, size: Size) {
        self.width = size.width;
        self.height = size.height;
        self.cells = vec![vec![Cell::default(); self.width]; self.height];
        self.needs_redraw = true;
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

        draw_end_of_buffer(view, &mut grid);
        draw_trailing_whitespace(view, &mut grid);

        redraw::Window::new(grid, view.primary_cursor)
    }
}

fn draw_end_of_buffer(view: &View, grid: &mut Vec<Vec<redraw::Cell>>) {
    for (line, row) in view.cells.iter().enumerate() {
        let is_empty = row
            .iter()
            .fold(true, |acc, cell| acc && matches!(cell, Cell::Empty));
        if is_empty {
            if let Some(rep) = view
                .display_options
                .replacements
                .get(&Replacement::BufferEnd)
            {
                grid[line][0] = rep.as_str().into();
            }
        }
    }
}

fn draw_trailing_whitespace(view: &View, grid: &mut Vec<Vec<redraw::Cell>>) {
    for (line, row) in view.cells.iter().enumerate() {}
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
