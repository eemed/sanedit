use std::cmp::min;

use sanedit_messages::redraw::{
    Cell, Component, Cursor, Diffable, Point, Redraw, Size, Statusline, Window,
};

use crate::ui::UIContext;

pub(crate) struct Grid {
    window: Rectangle<Window>,
    statusline: Rectangle<Statusline>,

    drawn: Vec<Vec<Cell>>,
    cursor: Cursor,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        let mut window = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let statusline = window.split_off(Split::top_size(1));

        Grid {
            window: Rectangle::new(Window::default(), window),
            statusline: Rectangle::new(Statusline::default(), statusline),

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
            _ => {} // Prompt(comp) => match comp {
                    //     Open(prompt) => self.prompt = Some(prompt),
                    //     Update(diff) => {
                    //         if let Some(ref mut prompt) = self.prompt {
                    //             prompt.update(diff);
                    //         }
                    //     }
                    //     Close => self.prompt = None,
                    // },
                    // Completion(comp) => match comp {
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
        if let Some(cur) = drawable.inner.cursor() {
            *cursor = cur;
            cursor.point = cursor.point + rect.position();
        }

        let top_left = rect.position();
        let mut grid = rect.grid();
        let mut g: Vec<&mut [Cell]> = grid.iter_mut().map(|v| v.as_mut_slice()).collect();
        drawable.inner.draw(&mut g);

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
    fn draw(&self, cells: &mut [&mut [Cell]]) {
        self.inner.draw(cells);
    }

    fn cursor(&self) -> Option<Cursor> {
        self.inner.cursor()
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
    fn draw(&self, cells: &mut [&mut [Cell]]);
    fn cursor(&self) -> Option<Cursor>;
}

impl Drawable for Window {
    fn draw(&self, cells: &mut [&mut [Cell]]) {
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

    fn cursor(&self) -> Option<Cursor> {
        Some(self.cursor)
    }
}

impl Drawable for Statusline {
    fn draw(&self, cells: &mut [&mut [Cell]]) {}

    fn cursor(&self) -> Option<Cursor> {
        None
    }
}
