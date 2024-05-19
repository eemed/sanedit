mod border;
mod canvas;
mod ccell;
mod completion;
mod drawable;
mod prompt;
mod rect;

use std::mem;

use sanedit_messages::redraw::{
    Cell, Completion, Component, Cursor, Diffable, Redraw, Size, StatusMessage, Statusline, Window,
};

use crate::{grid::completion::open_completion, ui::UIContext};

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
    completion: Option<Canvas<Completion>>,

    drawn: Vec<Vec<Cell>>,
    cursor: Option<Cursor>,
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
            completion: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: None,
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
            Completion(comp) => {
                log::info!("UI compl: {comp:?}");
                match comp {
                    Open(compl) => {
                        self.completion = Some(open_completion(self.window_area(), compl))
                    }
                    Update(diff) => {
                        if let Some(ref mut compl) = self.completion {
                            let drawable = compl.drawable();
                            drawable.update(diff);
                            let Size { width, height } = drawable.preferred_size();
                            compl.set_size(width, height);
                        }
                    }
                    Close => self.completion = None,
                }
            }
            _ => {} //
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

        self.cursor = None;
    }

    fn draw_drawable<D: Drawable>(
        drawable: &Canvas<D>,
        ctx: &UIContext,
        cursor: &mut Option<Cursor>,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        let rect = drawable.area().clone();
        if let Some(mut cur) = drawable.cursor(ctx) {
            cur.point = cur.point + rect.position();
            *cursor = Some(cur);
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

    pub fn draw(&mut self, ctx: &UIContext) -> (&Vec<Vec<Cell>>, Option<Cursor>) {
        self.clear();

        Self::draw_drawable(&self.window, ctx, &mut self.cursor, &mut self.drawn);
        Self::draw_drawable(&self.statusline, ctx, &mut self.cursor, &mut self.drawn);

        if let Some(ref prompt) = self.prompt {
            Self::draw_drawable(prompt, ctx, &mut self.cursor, &mut self.drawn);
        }

        if let Some(ref msg) = self.msg {
            Self::draw_drawable(msg, ctx, &mut self.cursor, &mut self.drawn);
        }

        if let Some(ref compl) = self.completion {
            Self::draw_drawable(compl, ctx, &mut self.cursor, &mut self.drawn);
        }

        (&self.drawn, self.cursor)
    }
}
