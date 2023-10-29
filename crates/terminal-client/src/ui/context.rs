use sanedit_messages::redraw::{Style, Theme, ThemeField};

#[derive(Debug)]
pub struct UIContext {
    pub theme: Theme,
    pub width: usize,
    pub height: usize,
    pub gutter_size: usize,
}

impl UIContext {
    pub fn new(width: usize, height: usize) -> UIContext {
        UIContext {
            theme: Theme::default(),
            width,
            height,
            gutter_size: 0,
        }
    }

    pub fn style(&self, field: &ThemeField) -> Style {
        self.theme.get(field)
    }
}
