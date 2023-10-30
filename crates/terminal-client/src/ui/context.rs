use sanedit_messages::redraw::{Style, Theme, ThemeField};

#[derive(Debug)]
pub struct UIContext {
    pub theme: Theme,
}

impl UIContext {
    pub fn new() -> UIContext {
        UIContext {
            theme: Theme::default(),
        }
    }

    pub fn style(&self, field: &ThemeField) -> Style {
        self.theme.get(field)
    }
}
