use std::sync::Arc;

use sanedit_messages::redraw::{Point, Style, Theme, ThemeField};

use crate::grid::Rect;

#[derive(Debug, Clone)]
pub(crate) struct UIContext {
    pub theme: Arc<Theme>,
    pub rect: Rect,
    pub client_in_focus: bool,
    pub cursor_position: Point,
}

impl UIContext {
    pub fn style(&self, field: ThemeField) -> Style {
        self.theme.get(field)
    }
}
