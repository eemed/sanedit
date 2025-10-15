use std::collections::VecDeque;

use sanedit_core::movement::prev_line_start;
use sanedit_core::{BufferRange, Char, Chars, DisplayOptions, Range, Replacement};
use sanedit_messages::redraw::{Point, Size};
use sanedit_utils::sorted_vec::SortedVec;
use unicode_width::UnicodeWidthChar;

use crate::editor::buffers::{Buffer, BufferId};
use crate::editor::syntax::{Span, SyntaxResult};

#[derive(Debug, Clone)]
pub(crate) enum Zone {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum Cell {
    #[default]
    Empty,
    Eof, // End of file where cursor can be placed
    Char {
        ch: Char,
    },
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

    pub fn len_in_buffer(&self) -> u64 {
        match self {
            Cell::Char { ch } => ch.len_in_buffer(),
            _ => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Cell::Empty)
    }

    pub fn is_virtual(&self) -> bool {
        match self {
            Cell::Empty => false,
            Cell::Eof => false,
            Cell::Char { ch } => ch.is_virtual(),
        }
    }
}

impl From<Char> for Cell {
    fn from(ch: Char) -> Self {
        Cell::Char { ch }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ViewSyntax {
    parse: SyntaxResult,
    total_changes_made: u32,
    bid: BufferId,
}

impl ViewSyntax {
    pub fn new(bid: BufferId, parse: SyntaxResult, total: u32) -> ViewSyntax {
        ViewSyntax {
            parse,
            total_changes_made: total,
            bid,
        }
    }

    pub fn total_changes_made(&self) -> u32 {
        self.total_changes_made
    }

    pub fn parsed_range(&self) -> BufferRange {
        self.parse.buffer_range
    }

    pub fn buffer_id(&self) -> BufferId {
        self.bid
    }

    pub fn spans(&self) -> &SortedVec<Span> {
        &self.parse.highlights
    }

    pub fn spans_mut(&mut self) -> &mut SortedVec<Span> {
        &mut self.parse.highlights
    }
}

/// View of the current window, used to draw the actual content sent to client
/// as well as implement movements which operate on visual information.
#[derive(Debug)]
pub(crate) struct View {
    /// buffer range
    range: BufferRange,
    /// Cells to hold drawn data
    cells: VecDeque<Vec<Cell>>,

    /// Display options which were used to draw this view
    pub options: DisplayOptions,
    needs_redraw: bool,

    pub(super) syntax: ViewSyntax,
}

impl View {
    pub fn new(width: usize, height: usize) -> View {
        View {
            range: Range::from(0..0),
            cells: Self::make_default_cells(width, height),
            options: DisplayOptions::new(width, height),
            needs_redraw: true,
            syntax: ViewSyntax::default(),
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
        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    pub fn width(&self) -> usize {
        self.options.width
    }

    pub fn height(&self) -> usize {
        self.options.height
    }

    fn draw_cells(&mut self, buf: &Buffer) {
        let slice = buf.slice(self.range.start..);
        let mut pos = 0;
        let mut line = 0;
        let mut col = 0;
        let mut is_eol = false;
        let mut graphemes = slice.graphemes_at(pos);

        let wrap = {
            let slice = buf.slice(..self.range.start);
            let mut graphemes = slice.graphemes_at(slice.len());
            let prev = graphemes.prev();
            if let Some(prev) = prev {
                let is_eol = prev.is_eol();
                !is_eol
            } else {
                false
            }
        };

        // Wrap mark on first line if not on eol boundary
        if wrap {
            if let Some(wrap) = self.options.replacements.get(&Replacement::Wrap) {
                let vch = Char::new_virtual(*wrap);
                let wrap_ch_width = vch.width();
                self.cells[line][col] = Cell::Char { ch: vch };
                col += wrap_ch_width;
            }
        }

        while let Some(grapheme) = graphemes.next() {
            let chars = Chars::new(&grapheme, col, &self.options);
            let ch_width: usize = chars.width();
            is_eol = chars.is_eol();

            // If we cannot fit this character, go to next line
            if col + ch_width > self.width() {
                if line + 1 >= self.height() {
                    break;
                }

                line += 1;
                col = 0;

                if let Some(wrap) = self.options.replacements.get(&Replacement::Wrap) {
                    let vch = Char::new_virtual(*wrap);
                    let wrap_ch_width = vch.width();
                    self.cells[line][col] = Cell::Char { ch: vch };
                    col += wrap_ch_width;
                }

                // Cannot fit this grapheme in any way
                if col + ch_width > self.width() {
                    break;
                }
            }

            match chars {
                Chars::Single { ch } => {
                    self.cells[line][col] = ch.into();
                }
                Chars::Multi { chars } => {
                    let mut w = col;
                    for ch in chars {
                        let oldw = w;
                        w += ch.width();
                        self.cells[line][col + oldw] = ch.into();
                    }
                }
            }

            // Increment pos, col once we have written the character to grid
            col += ch_width;
            pos += grapheme.len();

            // Goto next line if eol
            if is_eol {
                if line + 1 >= self.height() {
                    break;
                }
                line += 1;
                col = 0;
            }
        }

        // Add in EOF if we have space
        if pos == slice.len() {
            if !is_eol && col < self.width() {
                self.cells[line][col] = Cell::Eof;
            } else if is_eol && col == 0 {
                self.cells[line][0] = Cell::Eof;
            }
        }

        self.range = Range::from(self.range.start..self.range.start + pos);
    }

    pub fn redraw(&mut self, buf: &Buffer) {
        if self.needs_redraw {
            self.draw(buf);
        }
    }

    pub fn ensure_view_on_grapheme_boundary(&mut self, buf: &Buffer) {
        if self.start() > buf.len() {
            self.set_offset(buf.len());
        }

        // Ensure view in buf grapheme boundary
        let ppos = self.start();
        let slice = buf.slice(..);
        let mut graphemes = slice.graphemes_at(ppos);
        let npos = graphemes
            .next()
            .map(|slice| slice.start())
            .unwrap_or(buf.len());
        if ppos != npos {
            self.set_offset(npos);
            self.invalidate();
        }
    }

    fn draw(&mut self, buf: &Buffer) {
        self.needs_redraw = false;
        self.ensure_view_on_grapheme_boundary(buf);

        if self.range.start > buf.len() {
            self.range.start = buf.len();
            self.scroll_up_n(buf, (self.height() / 2) as u64);
        }

        self.clear();
        self.draw_cells(buf);
    }

    pub fn line_len_in_buffer(&self, line: usize) -> u64 {
        self.cells
            .get(line)
            .map(|row| row.iter().fold(0, |acc, cell| acc + cell.len_in_buffer()))
            .unwrap_or(0)
    }

    pub fn scroll_down_n(&mut self, buf: &Buffer, mut n: u64) {
        self.redraw(buf);

        if n >= self.height() as u64 {
            self.range.start = self.range.end;
            self.needs_redraw = true;
            return;
        }

        let mut line = 0;
        while n > 0 {
            let len = self.line_len_in_buffer(line);
            if len == 0 {
                break;
            }
            self.range.start += len;
            self.needs_redraw = true;
            n -= 1;
            line += 1;
        }
    }

    pub fn align_start(&mut self, mut width: usize, buf: &Buffer) {
        self.redraw(buf);

        if self.range.start == 0 {
            return;
        }

        width = width.min(self.width());

        // Go up until we find newlines,
        // but stop at a maximum if there are no lines.
        let mut pos = self.range.start;
        let min = pos.saturating_sub((self.height() * self.width()) as u64);
        let slice = buf.slice(min..pos);
        let mut graphemes = slice.graphemes_at(slice.len());
        let mut line_width = 0;
        let line_wrap_width = self
            .options
            .replacements
            .get(&Replacement::Wrap)
            .map(|ch| ch.width().unwrap_or(1))
            .unwrap_or(0);

        while let Some(grapheme) = graphemes.prev() {
            let ch = Chars::new(&grapheme, 0, &self.options);
            line_width += ch.width();
            // println!("Grapheme2: {:?} width: {line_wdth}, screen width: {}", String::from(&grapheme), self.width());

            if grapheme.is_eol() {
                break;
            }

            pos -= grapheme.len();

            if line_width > width {
                while let Some(g) = graphemes.next() {
                    let ch = Chars::new(&g, 0, &self.options);
                    line_width -= ch.width();
                    pos += g.len();

                    if line_width + line_wrap_width <= width {
                        break;
                    }
                }
                break;
            }
        }

        self.range.start = pos;
        self.needs_redraw = true;
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, mut n: u64) {
        self.redraw(buf);

        if self.range.start == 0 {
            return;
        }

        n = n.clamp(1, self.height().saturating_sub(1) as u64);

        // Go up until we find newlines,
        // but stop at a maximum if there are no lines.
        let mut pos = self.range.start;
        let min = pos.saturating_sub((self.height() * self.width()) as u64);
        let slice = buf.slice(min..pos);
        let mut graphemes = slice.graphemes_at(slice.len());
        let mut line_width = 0;
        let line_wrap_width = self
            .options
            .replacements
            .get(&Replacement::Wrap)
            .map(|ch| ch.width().unwrap_or(1))
            .unwrap_or(0);
        let width = self.width();
        // let mut wrap_symbol_width = 0; // No wrap symbol on first line ever
        // TODO columns not handled, the lines can jump anyway so maybe not necessary

        if let Some(grapheme) = graphemes.prev() {
            line_width += Chars::new(&grapheme, 0, &self.options).width();
            pos -= grapheme.len();
        }

        while let Some(grapheme) = graphemes.prev() {
            let ch = Chars::new(&grapheme, 0, &self.options);
            line_width += ch.width();

            pos = pos.saturating_sub(grapheme.len());

            if pos == 0 {
                break;
            }

            let is_eol = grapheme.is_eol();
            if is_eol {
                n = n.saturating_sub(1);

                if n == 0 {
                    pos += grapheme.len();
                    break;
                }

                line_width = ch.width();
            }

            if line_width > width {
                while let Some(g) = graphemes.next() {
                    let ch = Chars::new(&g, 0, &self.options);
                    line_width -= ch.width();
                    pos += g.len();

                    if line_width + line_wrap_width <= width {
                        graphemes.prev();
                        break;
                    }
                }
                n = n.saturating_sub(1);

                if n == 0 {
                    break;
                }

                line_width = 0;
            }
        }

        self.range.start = pos;
        self.needs_redraw = true;
    }

    pub fn range(&self) -> BufferRange {
        self.range
    }

    // Is used but not detected?
    #[allow(dead_code)]
    /// wether this view includes the buffer start
    pub fn at_start(&self) -> bool {
        self.range.start == 0
    }

    // Is used but not detected?
    #[allow(dead_code)]
    /// wether this view includes the buffer end
    pub fn at_end(&self) -> bool {
        self.cells[self.height() - 1]
            .iter()
            .all(|cell| matches!(cell, Cell::Empty))
    }

    pub fn set_offset(&mut self, offset: u64) {
        self.range.start = offset;
        self.range.end = offset;
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

    /// Return bufferrange of line in view
    pub fn line_at_pos(&self, pos: u64) -> Option<BufferRange> {
        let mut cur = self.range.start;
        for y in 0..self.cells.len() {
            let llen = self.line_len_in_buffer(y);

            if cur <= pos && pos < cur + llen {
                return Some(Range::from(cur..cur + llen));
            }

            cur += llen;
        }
        None
    }

    pub fn pos_at_point(&self, point: Point) -> Option<u64> {
        let mut pos = self.range.start;
        let mut ret_next = false;
        let mut last_virtual = false;

        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                match cell {
                    Cell::Eof => {
                        if ret_next {
                            return Some(pos);
                        }
                        if point.y == y && point.x == x {
                            return Some(pos);
                        }
                    }
                    Cell::Char { ch } => {
                        let first_virtual = !last_virtual && cell.is_virtual() && !cell.is_empty();
                        let ch_cell = !cell.is_virtual() && !cell.is_empty();
                        let is_valid_cursor_cell = first_virtual || ch_cell;
                        if ret_next && is_valid_cursor_cell {
                            return Some(pos);
                        }

                        if point.y == y && point.x == x {
                            if is_valid_cursor_cell {
                                return Some(pos);
                            } else if ch.is_virtual() {
                                ret_next = true;
                            }
                        }

                        pos += ch.len_in_buffer();
                        last_virtual = cell.is_virtual();
                    }
                    _ => {}
                }
            }
        }

        None
    }

    pub fn point_at_pos(&self, pos: u64) -> Option<Point> {
        let mut ret_next = false;
        let mut cpos = self.range.start;
        let mut last_virtual = false;

        for (y, row) in self.cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let first_virtual = !last_virtual && cell.is_virtual() && !cell.is_empty();
                let ch_cell = !cell.is_virtual() && !cell.is_empty();
                let is_valid_cursor_cell = first_virtual || ch_cell;

                if ret_next && is_valid_cursor_cell {
                    return Some(Point { x, y });
                }

                if cpos == pos {
                    if is_valid_cursor_cell {
                        return Some(Point { x, y });
                    } else if cell.is_virtual() {
                        ret_next = true;
                    }
                }

                cpos += cell.len_in_buffer();
                last_virtual = cell.is_virtual() && !cell.is_empty();
            }
        }

        None
    }

    pub fn start(&self) -> u64 {
        self.range.start
    }

    pub fn end(&self) -> u64 {
        self.range.end
    }

    pub fn contains(&self, pos: u64) -> bool {
        self.range.contains(&pos)
    }

    /// If distance is small we can scroll to the next position
    fn scroll_to(&mut self, pos: u64, buf: &Buffer) {
        while pos < self.start() {
            self.scroll_up_n(buf, 1);
            self.draw(buf);
        }

        while !self.is_visible(pos) {
            self.scroll_down_n(buf, 1);
            self.draw(buf);
        }
    }

    /// How much to move to be able to show pos in view
    fn offset_from(&self, pos: u64) -> u64 {
        if self.contains(pos) {
            return 0;
        }

        if pos < self.start() {
            self.start() - pos
        } else {
            pos - self.end()
        }
    }

    fn has_eof(&self) -> bool {
        let mut found_text = false;
        for row in self.cells.iter().rev() {
            for cell in row {
                match cell {
                    Cell::Eof => return true,
                    Cell::Empty => break,
                    _ => found_text = true,
                }
            }

            if found_text {
                break;
            }
        }

        false
    }

    /// Check wether the position is visible.
    pub fn is_visible(&self, pos: u64) -> bool {
        // pos is visible if it is in the buffer range or we are at the end of
        // the file and the view is currently showing that eof.
        self.contains(pos) || (pos == self.end() && self.has_eof())
    }

    /// Align view so that pos is shown
    pub fn view_to(&mut self, pos: u64, buf: &Buffer) {
        self.redraw(buf);

        // Scroll to position if its nearby
        let max = ((self.height() / 2) * self.width()) as u64;
        let offset = self.offset_from(pos);
        if offset <= max {
            self.scroll_to(pos, buf);
        }

        // Goto position and scroll it to the middle
        if !self.is_visible(pos) {
            self.set_offset(pos);
            self.scroll_up_n(buf, (self.height() / 2) as u64);
            self.draw(buf);
        }

        // Goto position and scroll until the whole line is visible
        if !self.is_visible(pos) {
            self.set_offset(pos);
            let min = pos.saturating_sub((self.width() * self.height()) as u64);
            let slice = &buf.slice(min..);
            self.range.start = min + prev_line_start(slice, slice.len());
            self.draw(buf);
        }

        // Just set the position
        if !self.is_visible(pos) {
            self.set_offset(pos);
            self.draw(buf);
        }
    }

    pub fn resize(&mut self, size: Size) {
        if size.width == self.width() && size.height == self.height() {
            return;
        }
        self.options.width = size.width;
        self.options.height = size.height;
        self.cells = Self::make_default_cells(size.width, size.height);
        self.needs_redraw = true;
    }

    pub fn syntax(&self) -> &ViewSyntax {
        &self.syntax
    }
}

impl Default for View {
    fn default() -> Self {
        View::new(0, 0)
    }
}
