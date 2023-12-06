mod border;
mod canvas;
mod ccell;
mod drawable;
mod prompt;
mod rect;

use std::{cmp::min, mem};

use sanedit_messages::redraw::{
    Cell, Component, Cursor, Diffable, Prompt, Redraw, Size, Source, StatusMessage, Statusline,
    Window,
};

use crate::{grid::prompt::PromptStyle, ui::UIContext};

pub(crate) use self::rect::{Rect, Split};
use self::{
    canvas::Canvas,
    ccell::CCell,
    drawable::Drawable,
    prompt::{open_prompt, CustomPrompt},
};

pub(crate) struct Grid {
    size: Size,
    window: Canvas<Window>,
    statusline: Canvas<Statusline>,
    // gutter: Option<Rectangle<()>>,
    prompt: Option<Canvas<CustomPrompt>>,
    msg: Option<Canvas<StatusMessage>>,

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
            window: Canvas::new(Window::default(), window),
            statusline: Canvas::new(Statusline::default(), statusline),
            prompt: None,
            msg: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: Cursor::default(),
        }
    }

    pub fn on_send_input(&mut self) {
        self.msg = None;
    }

    pub fn handle_redraw(&mut self, ctx: &UIContext, msg: Redraw) {
        use Component::*;
        use Redraw::*;

        let Size { width, height } = self.size;
        match msg {
            Window(comp) => match comp {
                Open(win) => *self.window.drawable() = win,
                Update(diff) => self.window.drawable().update(diff),
                Close => {}
            },
            Statusline(comp) => match comp {
                Open(status) => *self.statusline.drawable() = status,
                Update(diff) => self.statusline.drawable().update(diff),
                Close => {}
            },
            Prompt(comp) => match comp {
                Open(prompt) => self.prompt = Some(open_prompt(width, height, prompt)),
                Update(diff) => {
                    if let Some(ref mut prompt) = self.prompt {
                        prompt.drawable().prompt.update(diff);
                    }
                }
                Close => self.prompt = None,
            },
            StatusMessage(msg) => {
                let rect = Rect {
                    x: 0,
                    y: 0,
                    width,
                    height: 1,
                };
                self.msg = Some(Canvas::new(msg, rect));
            }
            _ => {} // Completion(comp) => match comp {
                    //     Open(compl) => self.completion = Some(compl),
                    //     Update(diff) => {
                    //         if let Some(ref mut compl) = self.completion {
                    //             compl.update(diff);
                    //         }
                    //     }
                    //     Close => self.completion = None,
                    // },
                    // LineNumbers(numbers) => {
                    //     let gutter = Gutter::new(numbers);
                    //     ctx.gutter_size = gutter.width();
                    //     self.gutter = gutter.into()
                    // }
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        // Keep externalized things
        let prompt = mem::take(&mut self.prompt);
        let msg = mem::take(&mut self.msg);
        let statusline = self.statusline.drawable().clone();

        *self = Grid::new(width, height);

        self.statusline = Canvas::new(statusline, self.statusline.area());

        if let Some(prompt) = prompt {
            let prompt = prompt.get().prompt;
            self.prompt = open_prompt(width, height, prompt).into();
        }

        if let Some(msg) = msg {
            let msg = msg.get();
            let canvas = Canvas::new(msg, self.statusline.area());
            self.msg = canvas.into();
        }
    }

    pub fn window_area(&self) -> Rect {
        self.window.area()
    }

    pub fn clear(&mut self) {
        for row in self.drawn.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    fn draw_drawable<D: Drawable>(
        drawable: &Canvas<D>,
        ctx: &UIContext,
        cursor: &mut Cursor,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        let rect = drawable.area().clone();
        if let Some(cur) = drawable.cursor(ctx) {
            *cursor = cur;
            cursor.point = cursor.point + rect.position();
        }

        let top_left = rect.position();
        let mut grid = rect.grid();
        let mut g: Vec<&mut [CCell]> = grid.iter_mut().map(|v| v.as_mut_slice()).collect();
        drawable.draw(ctx, &mut g);

        for (line, row) in grid.into_iter().enumerate() {
            for (col, cell) in row.into_iter().enumerate() {
                if cell.is_transparent {
                    continue;
                }
                let x = top_left.x + col;
                let y = top_left.y + line;
                cells[y][x] = cell.cell;
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
