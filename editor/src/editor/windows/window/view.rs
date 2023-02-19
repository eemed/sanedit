use std::collections::VecDeque;
use std::ops::Range;

use sanedit_buffer::piece_tree::{next_grapheme, prev_grapheme, PieceTreeSlice};
use sanedit_messages::redraw::{
    self, Point, Prompt, Redraw, Size, Statusline, Style, Theme, ThemeField,
};

use crate::common::char::{Char, DisplayOptions, GraphemeCategory, Replacement};
use crate::common::eol::EOL;
use crate::editor::buffers::Buffer;
use crate::editor::windows::window::{Cursor, Cursors};

use super::Window;

#[derive(Debug, Clone)]
pub(crate) enum Cell {
    Empty,
    EOF, // End of file where cursor can be placed
    Char {
        ch: Char,
        // style: Style,
    },
}

impl Cell {
    pub fn char(&self) -> Option<&Char> {
        match self {
            Cell::Empty => None,
            Cell::EOF => None,
            Cell::Char { ch } => Some(ch),
        }
    }

    pub fn width(&self) -> usize {
        match self {
            Cell::Empty => 0,
            Cell::EOF => 0,
            Cell::Char { ch } => ch.width(),
        }
    }

    pub fn grapheme_len(&self) -> usize {
        match self {
            Cell::Empty => 0,
            Cell::EOF => 0,
            Cell::Char { ch } => ch.grapheme_len(),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::Empty
    }
}

impl From<Char> for Cell {
    fn from(ch: Char) -> Self {
        Cell::Char { ch }
    }
}

/// View of the current window, used to draw the actual content sent to client
/// as well as implement movements which operate on visual information.
#[derive(Debug)]
pub(crate) struct View {
    /// buffer range
    range: Range<usize>,
    /// Cells to hold drawn data
    cells: VecDeque<Vec<Cell>>,
    /// Width of view
    width: usize,
    /// Height of view
    height: usize,
    /// Wether this view is out of date, and needs to be redrawn to
    /// represent current state.
    needs_redraw: bool,

    /// Display options which were used to draw this view
    pub options: DisplayOptions,
}

impl View {
    pub fn new(width: usize, height: usize) -> View {
        View {
            range: 0..0,
            cells: Self::make_default_cells(width, height),
            width,
            height,
            needs_redraw: true,
            options: DisplayOptions::default(),
        }
    }

    pub fn cells(&self) -> &VecDeque<Vec<Cell>> {
        &self.cells
    }

    fn make_default_cells(width: usize, height: usize) -> VecDeque<Vec<Cell>> {
        let row = vec![Cell::default(); width];
        let mut cells = VecDeque::with_capacity(height);
        for _ in 0..height {
            cells.push_back(row.clone());
        }
        cells
    }

    fn clear(&mut self) {
        self.cells = Self::make_default_cells(self.width, self.height);
        self.needs_redraw = true;
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn cursor_cell_pos(&mut self, cursor: &Cursor) -> Option<Point> {
        // Cursor is always on a character or at the end of file
        let mut pos = self.range.start;
        for (line, row) in self.cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                match cell {
                    Cell::Empty => {}
                    Cell::EOF => {
                        if cursor.pos() == pos {
                            return Some(Point { x: col, y: line });
                        }
                    }
                    Cell::Char { ch } => {
                        if cursor.pos() == pos {
                            return Some(Point { x: col, y: line });
                        }
                        pos += ch.grapheme_len();
                    }
                }
            }
        }

        None
    }

    fn draw_cells(&mut self, buf: &Buffer) {
        let slice = buf.slice(self.range.start..);
        let mut pos = 0;
        let mut line = 0;

        while line < self.height && !self.draw_line(&slice, line, &mut pos) {
            line += 1;
        }

        self.range = self.range.start..pos;
    }

    /// Draw a line into self.cells backwards, Fills a line backwards until EOL
    /// or the line is full.
    ///
    /// returns true if full redraw is required instead. This redraw should be
    /// started at pos
    ///
    /// EXPLANATION: if pos reaches 0 or EOL is found when drawing backwards a
    /// full redraw is needed , if the position we start to draw backwards is
    /// not EOL (so we have a wrapped line). Otherwise the line will be shorter eventhough
    /// more could fit to the line.
    fn draw_line_backwards(
        &mut self,
        slice: &PieceTreeSlice,
        line: usize,
        pos: &mut usize,
    ) -> bool {
        let mut graphemes = VecDeque::with_capacity(self.width);
        let mut is_eol = false;

        while let Some(grapheme) = prev_grapheme(&slice, *pos) {
            is_eol = EOL::is_eol(&grapheme);

            if is_eol && !graphemes.is_empty() {
                break;
            }

            // Must be recalculated because we are pushing elements to the front
            let line_len = graphemes.iter().fold(0, |col, grapheme| {
                let ch = Char::new(grapheme, col, &self.options);
                col + ch.width()
            });

            if line_len >= self.width {
                break;
            }

            *pos -= grapheme.len();
            graphemes.push_front(grapheme);
        }

        graphemes.into_iter().fold(0, |col, grapheme| {
            let ch = Char::new(&grapheme, col, &self.options);
            let width = ch.width();
            self.cells[line][col] = ch.into();
            col + width
        });

        // TODO return (pos == 0 || last was eol) && start was not eol
        *pos == 0 || is_eol
    }

    /// Draw a line into self.cells, returns true if EOF was written.
    ///
    /// If char does not fit to current line, no madeup representation will be
    /// made for it.
    fn draw_line(&mut self, slice: &PieceTreeSlice, line: usize, pos: &mut usize) -> bool {
        let mut col = 0;
        let mut is_eol = false;

        while let Some(grapheme) = next_grapheme(&slice, *pos) {
            is_eol = EOL::is_eol(&grapheme);
            let ch = Char::new(&grapheme, col, &self.options);
            let ch_width = ch.width();

            if col + ch_width > self.width {
                break;
            }

            self.cells[line][col] = ch.into();
            col += ch_width;
            *pos += grapheme.len();

            if is_eol {
                break;
            }
        }

        if !is_eol && *pos == slice.len() {
            self.cells[line][col] = Cell::EOF;
            return true;
        }

        false
    }

    pub fn draw(&mut self, win: &Window, buf: &Buffer) {
        log::info!("Draw view {}", self.needs_redraw);
        if !self.needs_redraw {
            return;
        }

        self.clear();
        self.draw_cells(buf);
        self.needs_redraw = false;
    }

    pub fn scroll_down(&mut self, win: &Window, buf: &Buffer) {
        let top_line_len = self
            .cells
            .get(0)
            .map(|row| row.iter().fold(0, |acc, cell| acc + cell.grapheme_len()))
            .unwrap_or(0);
        if top_line_len + self.range.start == buf.len() {
            return;
        }

        let _ = self.cells.pop_front();
        self.cells.push_back(vec![Cell::default(); self.width]);

        let slice = buf.slice(self.range.end..);
        let last_line = self.height - 1;
        let mut pos = 0;

        self.draw_line(&slice, last_line, &mut pos);
        self.range = self.range.start + top_line_len..self.range.end + pos;
    }

    pub fn scroll_up(&mut self, win: &Window, buf: &Buffer) {
        if self.range.start == 0 {
            return;
        }

        let last_line = self.cells.pop_back();
        let last_line_len = last_line
            .map(|row| row.iter().fold(0, |acc, cell| acc + cell.grapheme_len()))
            .unwrap_or(0);
        self.cells.push_front(vec![Cell::default(); self.width]);

        let slice = buf.slice(..self.range.start);
        let mut pos = self.range.start;

        self.needs_redraw = self.draw_line_backwards(&slice, 0, &mut pos);
        self.range = pos..self.range.end - last_line_len;
        self.draw(win, buf);
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// wether this view includes the buffer start
    pub fn at_start(&self) -> bool {
        self.range.start == 0
    }

    /// wether this view includes the buffer end
    pub fn at_end(&self) -> bool {
        self.cells[self.height - 1]
            .iter()
            .fold(true, |acc, cell| acc && matches!(cell, Cell::Empty))
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.range.start = offset;
        self.needs_redraw = true;
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
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
        let mut pos = self.range.start;
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

    pub fn point_at_pos(&self, pos: usize) -> Option<Point> {
        let mut cur = self.range.start;

        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if !matches!(cell, Cell::Empty) {
                    if cur == pos {
                        return Some(Point { x, y });
                    }
                }

                if let Cell::Char { ch } = cell {
                    cur += ch.grapheme_len();
                }
            }
        }

        None
    }

    /// Align view so that pos is shown
    pub fn view_to(&mut self, pos: usize, win: &Window, buf: &Buffer) {
        // // Make sure offset is inside buffer range
        // if win.offset > buf.len() {
        //     win.offset = buf.len();
        //     win.offset = scroll_prev_line((win, buf));
        // }

        // let mut view = WindowCells::new((win, buf));
        // let cursor = win.cursor.pos();
        // let Range { start, end } = view.buf_range;
        // if cursor >= start && end > cursor {
        //     return;
        // }

        // let cursor_was_ahead = end <= cursor;
        // let mut did_move = false;

        // // Set view to cursor line start
        // if cursor < start || end <= cursor {
        //     win.offset = start_of_line(buf, cursor);
        //     view = WindowCells::new((win, buf));
        //     did_move = true;
        // }

        // // If still not in view set view to cursor position
        // let Range { start, end } = view.buf_range;
        // if cursor < start || end <= cursor {
        //     win.offset = cursor;
        //     did_move = true;
        // }

        // // scroll until current line is at bottom
        // if did_move && cursor_was_ahead {
        //     let mut amount = win.size.height.get();

        //     // If last grapheme is eol we need to show one empty line
        //     // even though there is nothing there
        //     if cursor == buf.len() && ends_with_eol(buf) {
        //         amount -= 1;
        //     }

        //     for _ in 0..amount {
        //         win.offset = scroll_prev_line((&win, &buf));
        //     }
        // }
    }

    pub fn resize(&mut self, size: Size) {
        log::info!("Resize view {size:?}");
        if size.width == self.width && size.height == self.height {
            return;
        }
        self.width = size.width;
        self.height = size.height;
        self.cells = Self::make_default_cells(size.width, size.height);
        self.needs_redraw = true;
    }

    pub fn invalidate(&mut self) {
        self.needs_redraw = true;
    }
}

impl Default for View {
    fn default() -> Self {
        View::new(0, 0)
    }
}

#[cfg(test)]
mod test {
    use std::mem;

    use crate::editor::buffers::buffer::{Buffer, BufferId};

    use super::*;

    #[test]
    fn tabs() {
        let width = 80;
        let mut buf = Buffer::new();
        buf.append("\tHello\tWorld");

        let mut window = Window::new(BufferId::default(), width, 1);
        let mut view = mem::take(&mut window.view);
        view.draw(&buf, &window);

        // println!("{}", "-".repeat(width));
        // for row in &view.cells {
        //     for cell in row {
        //         print!("{}", cell.char().display());
        //     }
        //     println!("");
        // }
        // println!("{}", "-".repeat(width));
    }
}
