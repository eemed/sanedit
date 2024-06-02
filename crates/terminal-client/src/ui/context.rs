use std::sync::Arc;

use sanedit_messages::redraw::{Style, Theme, ThemeField};

use crate::grid::Rect;

#[derive(Debug, Clone)]
pub(crate) struct UIContext {
    pub theme: Arc<Theme>,
    pub rect: Rect,
}

impl UIContext {
    pub fn new() -> UIContext {
        UIContext {
            theme: Arc::new(Theme::default()),
            rect: Rect::new(0, 0, 0, 0),
        }
    }

    pub fn style(&self, field: ThemeField) -> Style {
        self.theme.get(field)
    }
}
