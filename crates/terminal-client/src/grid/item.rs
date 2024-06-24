use std::cmp::min;

use sanedit_messages::redraw::{Completion, Cursor, Size};

use crate::ui::UIContext;

use super::{
    completion::fit_completion,
    drawable::{DrawCursor, Drawable},
    CCell, Rect,
};

// TODO maybe use inner: Box<dyn Drawable>? so grid could handle
// Map<Type, GridItem>
/// Item that can be drawn to a part of the grid
pub(crate) struct GridItem<T>
where
    T: Drawable,
{
    inner: T,
    area: Rect,
}

impl<T: Drawable> GridItem<T> {
    pub fn new(t: T, rect: Rect) -> GridItem<T> {
        GridItem {
            inner: t,
            area: rect,
        }
    }

    pub fn area_mut(&mut self) -> &mut Rect {
        &mut self.area
    }

    pub fn area(&self) -> Rect {
        self.area.clone()
    }

    pub fn drawable(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn get(self) -> T {
        self.inner
    }
}

impl<T: Drawable> Drawable for GridItem<T> {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let mut ctx = ctx.clone();
        ctx.rect = self.area();
        self.inner.draw(&ctx, cells);
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        let mut ctx = ctx.clone();
        ctx.rect = self.area();
        self.inner.cursor(&ctx)
    }
}

impl GridItem<Completion> {
    pub fn update(&mut self, win: Rect) {
        let Size { width, height } = self.inner.preferred_size();
        let minw = min(width, win.width);
        if self.area.width < minw {
            self.area.width = minw;
        }
        if self.area.rightmost() > win.rightmost() {
            self.area.x -= self.area.rightmost() - win.rightmost();
        }

        self.area.height = min(height, win.height - win.y);

        if !win.includes(&self.area) {
            self.area = fit_completion(win, &self.inner);
        }
    }
}
