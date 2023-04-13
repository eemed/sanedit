use std::collections::VecDeque;
use std::ops::Range;

use sanedit_buffer::piece_tree::{next_grapheme, prev_grapheme, PieceTreeSlice};
use sanedit_messages::redraw::{Point, Size};

use crate::common::char::{Char, DisplayOptions};
use crate::common::eol::EOL;
use crate::common::movement::prev_line_start;
use crate::editor::buffers::Buffer;

#[derive(Debug, Clone)]
pub(crate) enum Cell {
    Empty,
    Continue, // Continuation of a previous char
    EOF,      // End of file where cursor can be placed
    Char { ch: Char },
}

impl Cell {
    pub fn char(&self) -> Option<&Char> {
        match self {
            Cell::Char { ch } => Some(ch),
            _ => None,
        }
    }

    pub fn width(&self) -> usize {
        match self {
            Cell::Char { ch } => ch.width(),
            _ => 0,
        }
    }

    pub fn grapheme_len(&self) -> usize {
        match self {
            Cell::Char { ch } => ch.grapheme_len(),
            _ => 0,
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

    /// Display options which were used to draw this view
    pub options: DisplayOptions,
    needs_redraw: bool,
}

impl View {
    pub fn new(width: usize, height: usize) -> View {
        View {
            range: 0..0,
            cells: Self::make_default_cells(width, height),
            width,
            height,
            options: DisplayOptions::default(),
            needs_redraw: true,
        }
    }

    pub fn invalidate(&mut self) {
        self.needs_redraw = true;
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
        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn draw_cells(&mut self, buf: &Buffer) {
        let slice = buf.slice(self.range.start..);
        let mut pos = 0;
        let mut line = 0;

        while line < self.height && !self.draw_line(&slice, line, &mut pos) {
            line += 1;
        }

        self.range = self.range.start..self.range.start + pos;
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

            for i in 1..ch_width {
                self.cells[line][col + i] = Cell::Continue;
            }

            col += ch_width;
            *pos += grapheme.len();

            if is_eol {
                break;
            }
        }

        if col < self.width && !is_eol && *pos == slice.len() {
            self.cells[line][col] = Cell::EOF;
            return true;
        }

        false
    }

    pub fn redraw(&mut self, buf: &Buffer) {
        if self.needs_redraw {
            self.draw(buf);
        }
    }

    fn draw(&mut self, buf: &Buffer) {
        self.needs_redraw = false;
        self.clear();
        self.draw_cells(buf);
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.cells
            .get(line)
            .map(|row| row.iter().fold(0, |acc, cell| acc + cell.grapheme_len()))
            .unwrap_or(0)
    }

    pub fn scroll_down_n(&mut self, buf: &Buffer, mut n: usize) {
        self.redraw(buf);
        if self.range.end >= buf.len() {
            return;
        }

        if n >= self.height {
            self.range.start = self.range.end;
            self.needs_redraw = true;
            return;
        }

        let mut line = 0;
        while n > 0 {
            let len = self.line_len(line);
            self.range.start += len;
            self.needs_redraw = true;
            n -= 1;
            line += 1;
        }
    }

    pub fn last_non_empty_line(&self) -> usize {
        let mut line = self.height.saturating_sub(1);
        loop {
            if let Some(_) = self.last_non_empty_cell(line) {
                return line;
            }

            line -= 1;
        }
    }

    pub fn line_width(&self, line: usize) -> usize {
        let mut width = 0;
        let line = &self.cells[line];

        for cell in line {
            if matches!(cell, Cell::Empty) {
                break;
            }

            width += cell.width();
        }

        width
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, mut n: usize) {
        self.redraw(buf);

        if n == 0 || self.range.start == 0 {
            return;
        }

        // Go up until we find newlines,
        // but stop at a maximum if there are no lines.
        let mut pos = self.range.start;
        let min = pos.saturating_sub(self.height * self.width);
        let slice = buf.slice(..pos);

        while let Some(grapheme) = prev_grapheme(&slice, pos) {
            let is_eol = EOL::is_eol(&grapheme);
            if is_eol && pos != self.range.start {
                n -= 1;

                if n == 0 {
                    break;
                }
            }

            pos -= grapheme.len();

            if min >= pos {
                break;
            }
        }

        self.range.start = pos;
        self.needs_redraw = true;
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
        let mut last_ch_pos = None;
        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                match cell {
                    Cell::EOF => {
                        if point.y == y && point.x == x {
                            return Some(pos);
                        }
                    }
                    Cell::Continue => {
                        if point.y == y && point.x == x {
                            return last_ch_pos;
                        }
                    }
                    Cell::Char { ch } => {
                        if point.y == y && point.x == x {
                            return Some(pos);
                        }
                        last_ch_pos = Some(pos);
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
                if !matches!(cell, Cell::Empty | Cell::Continue) {
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

    pub fn start(&self) -> usize {
        self.range.start
    }

    pub fn end(&self) -> usize {
        self.range.end
    }

    /// Align view so that pos is shown
    pub fn view_to(&mut self, pos: usize, buf: &Buffer) {
        self.redraw(buf);
        log::debug!("view to {pos} {:?}", self.range);

        // Make sure offset is inside buffer range
        if self.range.start > buf.len() {
            self.set_offset(buf.len());
        }

        // At end
        if self.range.end == buf.len() && pos == buf.len() {
            return;
        }

        if pos < self.start() || self.end() <= pos {
            self.set_offset(pos);
            self.scroll_up_n(buf, self.height() / 2);
            self.draw(buf);
        }

        if pos < self.start() || self.end() <= pos {
            self.set_offset(pos);
            let min = pos.saturating_sub(self.width() * self.height());
            let slice = &buf.slice(min..);
            // TODO slice.len() does not work as expected.
            self.range.start = min + prev_line_start(&slice, slice.len().saturating_sub(1));
            self.draw(buf);
        }

        if pos < self.start() || self.end() <= pos {
            self.set_offset(pos);
            self.draw(buf);
        }
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
}

impl Default for View {
    fn default() -> Self {
        View::new(0, 0)
    }
}
