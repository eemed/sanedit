use sanedit_messages::redraw::Cursor;

use crate::ui::UIContext;

use super::{drawable::Drawable, CCell, Rect};

pub(crate) struct Canvas<T>
where
    T: Drawable,
{
    inner: T,
    area: Rect,
}

impl<T: Drawable> Canvas<T> {
    pub fn new(t: T, rect: Rect) -> Canvas<T> {
        Canvas {
            inner: t,
            area: rect,
        }
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

impl<T: Drawable> Drawable for Canvas<T> {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        self.inner.draw(ctx, cells);
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        self.inner.cursor(ctx)
    }
}
