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
}

pub fn merge_cell_styles(styles: &[Style]) -> Style {
    styles
        .into_iter()
        .cloned()
        .fold(Style::default(), merge_2_cell_styles)
}

fn merge_2_cell_styles(mut one: Style, two: Style) -> Style {
    if let Some(bg) = two.bg {
        one.bg = Some(bg);
    }

    if let Some(fg) = two.fg {
        one.fg = Some(fg);
    }

    if let Some(s) = two.text_style {
        one.text_style = Some(s);
    }

    one
}
