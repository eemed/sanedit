use serde::{Deserialize, Serialize};

use std::mem;

use super::{Color, TextStyle};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Style {
    pub text_style: Option<TextStyle>,
    pub bg: Option<Color>,
    pub fg: Option<Color>,
}

impl Style {
    pub fn invert(&mut self) {
        mem::swap(&mut self.bg, &mut self.fg);
    }

    /// use self as a base style and apply overrides
    pub fn override_with(&mut self, style: &Style) {
        if let Some(bg) = style.bg {
            self.bg = Some(bg);
        }

        if let Some(fg) = style.fg {
            self.fg = Some(fg);
        }

        if let Some(s) = style.text_style {
            self.text_style = Some(s);
        }
    }
}
