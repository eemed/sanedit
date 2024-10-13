mod border;
pub(crate) mod ccell;
mod completion;
mod drawable;
mod items;
mod popup;
mod prompt;
mod rect;

use std::sync::Arc;

use completion::completion_rect;
use items::Kind;
use popup::popup_rect;
use prompt::prompt_rect;
use sanedit_messages::redraw::{
    completion::Completion, statusline::Statusline, window::Window, Cell, Component, Cursor,
    Diffable as _, Popup, PopupComponent, Redraw, Size, StatusMessage, Theme,
};

use crate::ui::UIContext;

pub(crate) use self::rect::{Rect, Split};
use self::{
    ccell::CCell,
    drawable::{DrawCursor, Drawable},
    items::CustomItems,
    prompt::CustomPrompt,
};

pub(crate) struct Grid {
    size: Size,
    window: (Window, Rect),
    statusline: (Statusline, Rect),
    prompt: Option<(CustomPrompt, Rect)>,
    msg: Option<(StatusMessage, Rect)>,
    completion: Option<(Completion, Rect)>,
    filetree: Option<(CustomItems, Rect)>,
    locations: Option<(CustomItems, Rect)>,
    popup: Option<(Popup, Rect)>,

    drawn: Vec<Vec<Cell>>,
    cursor: Option<Cursor>,
    pub theme: Arc<Theme>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        let mut me = Grid {
            size: Size { width, height },
            window: (Window::default(), Rect::default()),
            statusline: (Statusline::default(), Rect::default()),
            prompt: None,
            msg: None,
            completion: None,
            filetree: None,
            locations: None,
            popup: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: None,
            theme: Arc::new(Theme::default()),
        };
        me.refresh();
        me
    }

    pub fn on_send_input(&mut self) {
        self.msg = None;
    }

    pub fn handle_redraw(&mut self, msg: Redraw) -> RedrawResult {
        use Component::*;
        use Redraw::*;

        match msg {
            Window(comp) => match comp {
                Open(win) => self.window.0 = win,
                Update(diff) => self.window.0.update(diff),
                Close => {}
            },
            Statusline(comp) => match comp {
                Open(status) => self.statusline.0 = status,
                Update(diff) => self.statusline.0.update(diff),
                Close => {}
            },
            Prompt(comp) => match comp {
                Open(prompt) => {
                    self.prompt = Some((CustomPrompt::new(prompt), Rect::default()));
                    self.refresh_overlays();
                }
                Update(diff) => {
                    if let Some(ref mut custom_prompt) = self.prompt {
                        custom_prompt.0.prompt.update(diff);
                    }
                }
                Close => {
                    self.prompt = None;
                }
            },
            StatusMessage(msg) => {
                self.msg = Some((msg, self.statusline.1.clone()));
            }
            Completion(comp) => {
                match comp {
                    Open(compl) => self.completion = Some((compl, Rect::default())),
                    Update(diff) => {
                        if let Some(ref mut compl) = self.completion {
                            compl.0.update(diff);
                        }
                    }
                    Close => self.completion = None,
                }

                self.refresh_overlays();
            }
            Filetree(comp) => match comp {
                Open(ft) => {
                    self.filetree = Some((CustomItems::new(ft, Kind::Filetree), Rect::default()));
                    self.refresh();
                    return RedrawResult::Resized;
                }
                Update(diff) => {
                    if let Some(ref mut ft) = self.filetree {
                        ft.0.update(diff, ft.1);
                    }
                }
                Close => {
                    self.filetree = None;
                    self.refresh();
                    return RedrawResult::Resized;
                }
            },
            Locations(comp) => match comp {
                Open(locs) => {
                    self.locations =
                        Some((CustomItems::new(locs, Kind::Locations), Rect::default()));
                    self.refresh();
                    return RedrawResult::Resized;
                }
                Update(diff) => {
                    if let Some(ref mut locs) = self.locations {
                        locs.0.update(diff, locs.1);
                    }
                }
                Close => {
                    self.locations = None;
                    self.refresh();
                    return RedrawResult::Resized;
                }
            },
            Popup(popup) => match popup {
                PopupComponent::Open(popup) => {
                    self.popup = Some((popup, Rect::default()));
                    self.refresh_overlays();
                }
                PopupComponent::Close => {
                    self.popup = None;
                }
            },
        }

        RedrawResult::Ok
    }

    fn screen(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        }
    }

    fn refresh_overlays(&mut self) {
        let screen = self.screen();
        let win = self.window();

        if let Some(compl) = &mut self.completion {
            let new = completion_rect(win, &mut compl.0);
            // Update only if bigger or old old does not fit
            if !compl.1.includes(&new) || !win.includes(&compl.1) {
                compl.1 = new
            }
        }

        if let Some(prompt) = &mut self.prompt {
            prompt.1 = prompt_rect(screen, &mut prompt.0);
        }

        if let Some(popup) = &mut self.popup {
            popup.1 = popup_rect(screen, win, &popup.0);
        }
    }

    /// Calculate locations for all
    pub fn refresh(&mut self) {
        let mut window = self.screen();
        self.statusline.1 = window.split_off(Split::top_size(1));

        // Message same as statusline
        if let Some(msg) = &mut self.msg {
            msg.1 = self.statusline.1;
        }

        // Filetree if present
        if let Some(ft) = &mut self.filetree {
            ft.1 = window.split_off(Split::left_size((window.width / 6).clamp(40, 50)));
        }

        if let Some(loc) = &mut self.locations {
            loc.1 = window.split_off(Split::bottom_size(15));
        }

        self.window.1 = window;
        self.refresh_overlays();
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.size.width = width;
        self.size.height = height;
        self.drawn = vec![vec![Cell::default(); width]; height];
        self.refresh();
    }

    pub fn window(&self) -> Rect {
        self.window.1
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
        drawable: &D,
        rect: Rect,
        theme: &Arc<Theme>,
        cursor: &mut Option<Cursor>,
        cells: &mut [Vec<Cell>],
    ) {
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
        Self::draw_drawable(
            &self.window.0,
            self.window.1,
            t,
            &mut self.cursor,
            &mut self.drawn,
        );
        Self::draw_drawable(
            &self.statusline.0,
            self.statusline.1,
            t,
            &mut self.cursor,
            &mut self.drawn,
        );

        if let Some((loc, rect)) = &self.locations {
            Self::draw_drawable(loc, *rect, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some((ft, rect)) = &self.filetree {
            Self::draw_drawable(ft, *rect, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some((prompt, rect)) = &self.prompt {
            Self::draw_drawable(prompt, *rect, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some((msg, rect)) = &self.msg {
            Self::draw_drawable(msg, *rect, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some((compl, rect)) = &self.completion {
            Self::draw_drawable(compl, *rect, t, &mut self.cursor, &mut self.drawn);
        }

        if let Some((popup, rect)) = &self.popup {
            Self::draw_drawable(popup, *rect, t, &mut self.cursor, &mut self.drawn);
        }

        (&self.drawn, self.cursor)
    }
}

pub(crate) enum RedrawResult {
    Ok,
    Resized,
}
