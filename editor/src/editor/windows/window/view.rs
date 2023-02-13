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
    display_options: DisplayOptions,

    /// Wether this view has changed and should be sent to clients again
    has_changed: bool,
}

impl View {
    pub fn new(width: usize, height: usize) -> View {
        View {
            range: 0..0,
            cells: Self::make_default_cells(width, height),
            width,
            height,
            needs_redraw: true,
            display_options: DisplayOptions::default(),
            has_changed: true,
        }
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

    /// Advance line and col in the grid by amount, May advance past self.height
    /// lines, but will not advance past self.width columns
    fn grid_advance(&mut self, line: &mut usize, col: &mut usize, amount: usize) {
        for _ in 0..amount {
            *col += 1;

            if *col == self.width {
                *line += 1;
                *col = 0;
            }
        }
    }

    // Tries to advance line and col backwards, if 0,0 is encoutered returns
    // false
    fn grid_advance_backwards(&mut self, line: &mut usize, col: &mut usize, amount: usize) -> bool {
        for _ in 0..amount {
            if *col == 0 {
                if *line == 0 {
                    return false;
                }

                *line = line.saturating_sub(1);
                *col = self.width.saturating_sub(1);
            }

            *col -= 1;
        }

        true
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
        let mut col = 0;

        while pos != buf.len() && line < self.height {
            self.draw_line(&slice, &mut pos, &mut line, &mut col);
        }

        if pos == buf.len() {
            self.cells[line][col] = Cell::EOF;
        }

        self.range = self.range.start..pos;
    }

    /// Draw a line into self.cells backwards
    fn draw_line_backwards(
        &mut self,
        slice: &PieceTreeSlice,
        pos: &mut usize,
        line: &mut usize,
        col: &mut usize,
    ) {
        let mut buf = Vec::with_capacity(self.width);
        let cur = *line;
        let start_line = *line;
        let start_pos = *pos;

        while let Some(grapheme) = prev_grapheme(&slice, *pos) {
            let grapheme_len = grapheme.len();
            let is_eol = EOL::is_eol(&grapheme);
            let ch = Char::new(grapheme, *col, &self.display_options);
            let ch_width = ch.width();
            let cell = ch.into();

            if is_eol && *pos != start_pos {
                break;
            }

            buf.push(cell);
            *pos -= grapheme_len;

            if !self.grid_advance_backwards(line, col, ch_width) || cur != *line {
                break;
            }
        }

        // TODO Realign buf if it contains tabs it may shrink
        // when more stuff is added

        for (i, cell) in buf.into_iter().rev().enumerate() {
            self.cells[start_line][i] = cell;
        }
    }

    /// Draw a line into self.cells
    fn draw_line(
        &mut self,
        slice: &PieceTreeSlice,
        pos: &mut usize,
        line: &mut usize,
        col: &mut usize,
    ) {
        let cur = *line;

        while let Some(grapheme) = next_grapheme(&slice, *pos) {
            let grapheme_len = grapheme.len();
            let is_eol = EOL::is_eol(&grapheme);
            let ch = Char::new(grapheme, *col, &self.display_options);
            let ch_width = ch.width();
            let cell = ch.into();

            self.cells[*line][*col] = cell;
            *pos += grapheme_len;

            if is_eol {
                *line += 1;
                *col = 0;
            } else {
                self.grid_advance(line, col, ch_width);
            }

            if cur != *line {
                break;
            }
        }
    }

    pub fn redraw(&mut self, win: &Window, buf: &Buffer) {
        if !self.needs_redraw {
            return;
        }

        self.display_options = win.options.display.clone();
        let cursors = win.cursors();
        self.clear();
        self.draw_cells(buf);
        self.needs_redraw = false;
        self.has_changed = true;
    }

    pub fn scroll_down(&mut self, win: &Window, buf: &Buffer) {
        let first_line_len = self
            .cells
            .get(0)
            .map(|row| row.iter().fold(0, |acc, cell| acc + cell.grapheme_len()))
            .unwrap_or(0);
        if first_line_len + self.range.start == buf.len() {
            return;
        }

        self.has_changed = true;
        let _ = self.cells.pop_front();
        self.cells.push_back(vec![Cell::default(); self.width]);

        let slice = buf.slice(self.range.end..);
        let last_line = self.height - 1;
        let mut pos = 0;
        let mut line = last_line;
        let mut col = 0;

        self.draw_line(&slice, &mut pos, &mut line, &mut col);
        self.range = self.range.start + first_line_len..self.range.end + pos;
    }

    pub fn scroll_up(&mut self, win: &Window, buf: &Buffer) {
        if self.range.start == 0 {
            return;
        }

        self.has_changed = true;
        let last_line = self.cells.pop_back();
        let last_line_len = last_line
            .map(|row| row.iter().fold(0, |acc, cell| acc + cell.grapheme_len()))
            .unwrap_or(0);
        self.cells.push_front(vec![Cell::default(); self.width]);

        let slice = buf.slice(..self.range.start);
        let mut pos = self.range.start;
        let mut line = 0;
        let mut col = self.width.saturating_sub(1);

        self.draw_line_backwards(&slice, &mut pos, &mut line, &mut col);
        self.range = pos..self.range.end - last_line_len;
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
    pub fn align_to_show(&mut self, pos: usize, win: &Window, buf: &Buffer) {
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
        self.width = size.width;
        self.height = size.height;
        self.cells = Self::make_default_cells(size.width, size.height);
        self.needs_redraw = true;
    }

    pub fn invalidate(&mut self) {
        self.needs_redraw = true;
    }

    pub fn draw_window(
        &mut self,
        win: &Window,
        buf: &Buffer,
        theme: &Theme,
    ) -> Option<redraw::Window> {
        self.redraw(win, buf);

        // if !self.has_changed {
        //     return None;
        // }

        let win = draw_window(self, win, theme);
        self.has_changed = false;
        Some(win)
    }
}

impl Default for View {
    fn default() -> Self {
        View::new(0, 0)
    }
}

fn draw_window(view: &View, win: &Window, theme: &Theme) -> redraw::Window {
    let def = theme
        .get(ThemeField::Default.into())
        .unwrap_or(Style::default());
    let mut grid = vec![
        vec![
            redraw::Cell {
                text: " ".into(),
                style: def
            };
            view.width()
        ];
        view.height()
    ];

    for (line, row) in view.cells.iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            grid[line][col] = cell.char().map(|ch| ch.display()).unwrap_or(" ").into();
        }
    }

    draw_end_of_buffer(&mut grid, view, theme);
    draw_trailing_whitespace(&mut grid, view, theme);

    let cursor = view
        .point_at_pos(win.cursors().primary().pos())
        .expect("cursor not at view");
    redraw::Window::new(grid, cursor)
}

fn draw_end_of_buffer(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    let eob = theme
        .get(ThemeField::EndOfBuffer.into())
        .unwrap_or(Style::default());
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
                grid[line][0] = redraw::Cell {
                    text: rep.as_str().into(),
                    style: eob,
                };
            }
        }
    }
}

fn draw_trailing_whitespace(grid: &mut Vec<Vec<redraw::Cell>>, view: &View, theme: &Theme) {
    for (line, row) in view.cells.iter().enumerate() {}
}

pub(crate) fn draw_statusline(win: &Window, buf: &Buffer) -> Statusline {
    let line = match win.message.as_ref() {
        Some(msg) => format!("{}", msg.message),
        None => format!("{}", buf.name()),
    };

    Statusline::new(line.as_str())
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
        view.redraw(&buf, &window);

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
