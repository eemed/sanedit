mod border;
mod ccell;
mod completion;
mod drawable;
mod item;
mod items;
mod prompt;
mod rect;

use std::{mem, sync::Arc};

use sanedit_messages::redraw::{
    Cell, Completion, Component, Cursor, Diffable, Redraw, Size, StatusMessage, Statusline, Theme,
    Window,
};

use crate::{
    grid::{
        completion::open_completion,
        items::{open_filetree, open_locations},
    },
    ui::UIContext,
};

pub(crate) use self::rect::{Rect, Split};
use self::{
    ccell::CCell,
    drawable::{DrawCursor, Drawable},
    item::GridItem,
    items::CustomItems,
    prompt::{open_prompt, CustomPrompt},
};

pub(crate) struct Grid {
    size: Size,
    window: GridItem<Window>,
    statusline: GridItem<Statusline>,
    prompt: Option<GridItem<CustomPrompt>>,
    msg: Option<GridItem<StatusMessage>>,
    completion: Option<GridItem<Completion>>,
    filetree: Option<GridItem<CustomItems>>,
    locations: Option<GridItem<CustomItems>>,

    drawn: Vec<Vec<Cell>>,
    cursor: Option<Cursor>,
    pub theme: Arc<Theme>,
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
            window: GridItem::new(Window::default(), window),
            statusline: GridItem::new(Statusline::default(), statusline),
            prompt: None,
            msg: None,
            completion: None,
            filetree: None,
            locations: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: None,
            theme: Arc::new(Theme::default()),
        }
    }

    pub fn on_send_input(&mut self) {
        self.msg = None;
    }

    pub fn handle_redraw(&mut self, msg: Redraw) -> RedrawResult {
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
                self.msg = Some(GridItem::new(msg, rect));
            }
            Completion(comp) => match comp {
                Open(compl) => self.completion = Some(open_completion(self.window_area(), compl)),
                Update(diff) => {
                    if let Some(ref mut compl) = self.completion {
                        compl.drawable().update(diff);
                        compl.update(self.window.area());
                    }
                }
                Close => self.completion = None,
            },
            Filetree(comp) => match comp {
                Open(ft) => {
                    let items = open_filetree(self.window.area(), ft);
                    let ft_area = items.area();
                    self.filetree = Some(items);

                    let warea = self.window.area_mut();
                    warea.width -= ft_area.width;
                    warea.x += ft_area.width;

                    return RedrawResult::Resized;
                }
                Update(diff) => {
                    if let Some(ref mut ft) = self.filetree {
                        ft.drawable().items.update(diff);
                        ft.update();
                    }
                }
                Close => {
                    let ft_area = self
                        .filetree
                        .as_ref()
                        .expect("Closing filetree that is not open")
                        .area();

                    let warea = self.window.area_mut();
                    warea.width += ft_area.width;
                    warea.x -= ft_area.width;

                    self.filetree = None;
                    return RedrawResult::Resized;
                }
            },
            Locations(comp) => match comp {
                Open(ft) => self.locations = Some(open_locations(self.window.area(), ft)),
                Update(diff) => {
                    if let Some(ref mut ft) = self.locations {
                        ft.drawable().items.update(diff);
                        ft.update();
                    }
                }
                Close => self.locations = None,
            },
            _ => {}
        }

        RedrawResult::Ok
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        // Keep externalized things
        let theme = self.theme.clone();
        let prompt = mem::take(&mut self.prompt);
        let msg = mem::take(&mut self.msg);
        let statusline = self.statusline.drawable().clone();
        let ft = mem::take(&mut self.filetree);

        *self = Grid::new(width, height);

        self.theme = theme;
        self.statusline = GridItem::new(statusline, self.statusline.area());

        if let Some(prompt) = prompt {
            let prompt = prompt.get().prompt;
            self.prompt = open_prompt(width, height, prompt).into();
        }

        if let Some(msg) = msg {
            let msg = msg.get();
            let item = GridItem::new(msg, self.statusline.area());
            self.msg = item.into();
        }

        if let Some(ft) = ft {
            let ft = ft.get();
            let item = open_filetree(self.window_area(), ft.items);
            self.filetree = item.into();
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
        drawable: &GridItem<D>,
        theme: &Arc<Theme>,
        cursor: &mut Option<Cursor>,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        let rect = drawable.area().clone();
        let ctx = UIContext {
            theme: theme.clone(),
            rect,
        };

        match drawable.cursor(&ctx) {
            DrawCursor::Hide => *cursor = None,
            DrawCursor::Show(mut cur) => {
                cur.point = cur.point + rect.position();
                *cursor = Some(cur);
            }
            DrawCursor::Ignore => {}
        }

        let top_left = rect.position();
        let mut grid = rect.grid();
        let mut g: Vec<&mut [CCell]> = grid.iter_mut().map(|v| v.as_mut_slice()).collect();
        drawable.draw(&ctx, &mut g);

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

    pub fn draw(&mut self) -> (&Vec<Vec<Cell>>, Option<Cursor>) {
        self.clear();

        let t = &self.theme;
        Self::draw_drawable(&self.window, t, &mut self.cursor, &mut self.drawn);
        Self::draw_drawable(&self.statusline, t, &mut self.cursor, &mut self.drawn);

        if let Some(ref loc) = self.locations {
            Self::draw_drawable(loc, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some(ref ft) = self.filetree {
            Self::draw_drawable(ft, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some(ref prompt) = self.prompt {
            Self::draw_drawable(prompt, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some(ref msg) = self.msg {
            Self::draw_drawable(msg, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some(ref compl) = self.completion {
            Self::draw_drawable(compl, t, &mut self.cursor, &mut self.drawn);
        }

        (&self.drawn, self.cursor)
    }
}

pub(crate) enum RedrawResult {
    Ok,
    Resized,
}
