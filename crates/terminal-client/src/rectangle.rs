use std::cmp::min;

use sanedit_messages::redraw::{
    Cell, Component, Cursor, CursorShape, Diffable, IntoCells, Point, Prompt, Redraw, Size,
    Statusline, Style, ThemeField, Window,
};

use crate::ui::UIContext;

pub(crate) struct Grid {
    size: Size,
    window: Rectangle<Window>,
    statusline: Rectangle<Statusline>,
    // gutter: Option<Rectangle<()>>,
    prompt: Option<Rectangle<Prompt>>,

    drawn: Vec<Vec<Cell>>,
    cursor: Cursor,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        let size = Size { width, height };
        let mut window = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let statusline = window.split_off(Split::top_size(1));

        Grid {
            size,
            window: Rectangle::new(Window::default(), window),
            statusline: Rectangle::new(Statusline::default(), statusline),
            prompt: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: Cursor::default(),
        }
    }

    pub fn handle_redraw(&mut self, ctx: &UIContext, msg: Redraw) {
        use Component::*;
        use Redraw::*;

        match msg {
            Window(comp) => match comp {
                Open(win) => self.window.inner = win,
                Update(diff) => self.window.inner.update(diff),
                Close => {}
            },
            Statusline(comp) => match comp {
                Open(status) => self.statusline.inner = status,
                Update(diff) => self.statusline.inner.update(diff),
                Close => {}
            },
            Prompt(comp) => match comp {
                Open(prompt) => {
                    self.prompt = Some(Rectangle::new(
                        prompt,
                        Rect::centered(self.size.width, self.size.height),
                    ));
                }
                Update(diff) => {
                    if let Some(ref mut prompt) = self.prompt {
                        prompt.inner.update(diff);
                    }
                }
                Close => self.prompt = None,
            },
            _ => {} // Completion(comp) => match comp {
                    //     Open(compl) => self.completion = Some(compl),
                    //     Update(diff) => {
                    //         if let Some(ref mut compl) = self.completion {
                    //             compl.update(diff);
                    //         }
                    //     }
                    //     Close => self.completion = None,
                    // },
                    // StatusMessage(msg) => self.msg = Some(msg),
                    // LineNumbers(numbers) => {
                    //     let gutter = Gutter::new(numbers);
                    //     ctx.gutter_size = gutter.width();
                    //     self.gutter = gutter.into()
                    // }
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {}

    pub fn window_rect(&self) -> Rect {
        self.window.rect.clone()
    }

    pub fn clear(&mut self) {
        for row in self.drawn.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    fn draw_drawable<D: Drawable>(
        drawable: &Rectangle<D>,
        ctx: &UIContext,
        cursor: &mut Cursor,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        let rect = drawable.rect.clone();
        if let Some(cur) = drawable.cursor(ctx) {
            *cursor = cur;
            cursor.point = cursor.point + rect.position();
        }

        let top_left = rect.position();
        let mut grid = rect.grid();
        let mut g: Vec<&mut [Cell]> = grid.iter_mut().map(|v| v.as_mut_slice()).collect();
        drawable.draw(ctx, &mut g);

        for (line, row) in grid.into_iter().enumerate() {
            for (col, cell) in row.into_iter().enumerate() {
                let x = top_left.x + col;
                let y = top_left.y + line;
                cells[y][x] = cell;
            }
        }
    }

    pub fn draw(&mut self, ctx: &UIContext) -> (&Vec<Vec<Cell>>, Cursor) {
        self.clear();

        Self::draw_drawable(&self.window, ctx, &mut self.cursor, &mut self.drawn);
        Self::draw_drawable(&self.statusline, ctx, &mut self.cursor, &mut self.drawn);

        if let Some(ref prompt) = self.prompt {
            Self::draw_drawable(prompt, ctx, &mut self.cursor, &mut self.drawn);
        }

        (&self.drawn, self.cursor)
    }
}

pub(crate) struct Rectangle<T>
where
    T: Drawable,
{
    inner: T,
    rect: Rect,
}

impl<T: Drawable> Drawable for Rectangle<T> {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [Cell]]) {
        self.inner.draw(ctx, cells);
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        self.inner.cursor(ctx)
    }
}

impl<T: Drawable> Rectangle<T> {
    pub fn new(t: T, rect: Rect) -> Rectangle<T> {
        Rectangle { inner: t, rect }
    }
}

pub(crate) enum SplitPoint {
    Percentage(usize),
    Size(usize),
}

impl SplitPoint {
    pub fn get(&self, size: usize) -> usize {
        match self {
            SplitPoint::Percentage(p) => (size * p) / 100,
            SplitPoint::Size(s) => min(*s, size),
        }
    }
}

pub(crate) enum Split {
    Top(SplitPoint),
    Bottom(SplitPoint),
    Left(SplitPoint),
    Right(SplitPoint),
}

impl Split {
    pub fn top_size(size: usize) -> Split {
        Split::Top(SplitPoint::Size(size))
    }

    pub fn bottom_size(size: usize) -> Split {
        Split::Bottom(SplitPoint::Size(size))
    }

    pub fn left_size(size: usize) -> Split {
        Split::Left(SplitPoint::Size(size))
    }

    pub fn right_size(size: usize) -> Split {
        Split::Right(SplitPoint::Size(size))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn centered(width: usize, height: usize) -> Rect {
        let width = width / 2;
        let height = height / 2;
        let x = width / 2;
        let y = height / 2;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn position(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    pub fn grid(&self) -> Vec<Vec<Cell>> {
        vec![vec![Cell::default(); self.width]; self.height]
    }

    pub fn split_off(&mut self, split: Split) -> Rect {
        match split {
            Split::Top(split) => {
                let amount = split.get(self.height);
                self.y += amount;
                self.height -= amount;

                Rect {
                    x: self.x,
                    y: self.y - amount,
                    width: self.width,
                    height: amount,
                }
            }
            Split::Bottom(split) => {
                let amount = split.get(self.height);
                self.height -= amount;

                Rect {
                    x: self.x,
                    y: self.y + self.height,
                    width: self.width,
                    height: amount,
                }
            }
            Split::Left(split) => {
                let amount = split.get(self.width);
                self.x += amount;
                self.width -= amount;

                Rect {
                    x: self.x - amount,
                    y: self.y,
                    width: amount,
                    height: self.height,
                }
            }
            Split::Right(split) => {
                let amount = split.get(self.width);
                self.width -= amount;

                Rect {
                    x: self.x + self.width,
                    y: self.y,
                    width: amount,
                    height: self.height,
                }
            }
        }
    }
}

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [Cell]]);
    fn cursor(&self, ctx: &UIContext) -> Option<Cursor>;
}

impl Drawable for Window {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [Cell]]) {
        let width = min(
            cells.get(0).map(|c| c.len()).unwrap_or(0),
            self.cells.get(0).map(|c| c.len()).unwrap_or(0),
        );
        let height = min(cells.len(), self.cells.len());

        for x in 0..width {
            for y in 0..height {
                cells[y][x] = self.cells[y][x].clone();
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        Some(self.cursor)
    }
}

impl Drawable for Statusline {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [Cell]]) {
        let style = ctx.style(ThemeField::Statusline);
        let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
        for (i, cell) in into_cells_with_theme_pad_with(&self.line, &style, width)
            .into_iter()
            .enumerate()
        {
            cells[0][i] = cell;
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        None
    }
}

impl Drawable for Prompt {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [Cell]]) {
        let size = size(cells);
        let default_style = ctx.theme.get(ThemeField::PromptDefault);
        let msg_style = ctx.theme.get(ThemeField::PromptMessage);
        let input_style = ctx.theme.get(ThemeField::PromptUserInput);
        let pcompl = ctx.theme.get(ThemeField::PromptCompletion);

        set_style(cells, pcompl);
        let mut message = into_cells_with_style(&self.message, msg_style);
        let colon = into_cells_with_style(": ", msg_style);
        let input = into_cells_with_style(&self.input, input_style);
        message.extend(colon);
        message.extend(input);
        pad_line(&mut message, default_style, size.width);
        put_line(message, 0, cells);

        self.options.iter().enumerate().for_each(|(i, opt)| {
            let field = if Some(i) == self.selected {
                ThemeField::PromptCompletionSelected
            } else {
                ThemeField::PromptCompletion
            };
            let style = ctx.style(field);
            put_line(
                into_cells_with_style_pad(opt, style, size.width),
                i + 1,
                cells,
            );
        });
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        let cursor_col = {
            let input_cells_before_cursor = self.input[..self.cursor].into_cells().len();
            let msg_len = self.message.into_cells().len();
            let extra = 2; // ": "
            msg_len + extra + input_cells_before_cursor
        };
        let style = ctx.theme.get(ThemeField::Default);
        Some(Cursor {
            bg: style.fg,
            fg: style.bg,
            point: Point {
                x: cursor_col,
                y: 0,
            },
            shape: CursorShape::Line(true),
        })
    }
}

fn into_cells_with_style(string: &str, style: Style) -> Vec<Cell> {
    let mut cells = string.into_cells();
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

fn into_cells_with_style_pad(string: &str, style: Style, width: usize) -> Vec<Cell> {
    let mut cells = into_cells_with_style(string, style);
    pad_line(&mut cells, style, width);
    cells
}

fn into_cells_with_theme_pad_with(string: &str, style: &Style, width: usize) -> Vec<Cell> {
    let mut cells = into_cells_with_theme_with(string, style);
    pad_line(&mut cells, style.clone(), width);
    cells
}

fn into_cells_with_theme_with(string: &str, style: &Style) -> Vec<Cell> {
    let mut cells = string.into_cells();
    cells.iter_mut().for_each(|cell| cell.style = style.clone());
    cells
}

fn pad_line(cells: &mut Vec<Cell>, style: Style, width: usize) {
    while cells.len() < width {
        cells.push(Cell::with_style(style.clone()));
    }

    while cells.len() > width {
        cells.pop();
    }
}

fn size(cells: &mut [&mut [Cell]]) -> Size {
    let height = cells.len();
    let width = cells.get(0).map(|line| line.len()).unwrap_or(0);

    Size { width, height }
}

fn put_line(line: Vec<Cell>, pos: usize, target: &mut [&mut [Cell]]) {
    for (i, cell) in line.into_iter().enumerate() {
        target[pos][i] = cell;
    }
}

fn set_style(target: &mut [&mut [Cell]], style: Style) {
    for line in target.iter_mut() {
        for cell in line.iter_mut() {
            cell.style = style.clone();
        }
    }
}
