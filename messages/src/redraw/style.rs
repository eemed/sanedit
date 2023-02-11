use serde::{Deserialize, Serialize};

use std::mem;

use super::{Color, TextStyle};

#[derive(Serialize, Deserialize,Debug, PartialEq, Eq, Clone, Copy, Default)]
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

pub fn merge_cell_styles(styles: &[Option<Style>]) -> Option<Style> {
    styles.into_iter().cloned().fold(None, merge_2_cell_styles)
}

fn merge_2_cell_styles(one: Option<Style>, two: Option<Style>) -> Option<Style> {
    if one.is_none() {
        return two;
    }
    if two.is_none() {
        return one;
    }

    let mut one = one.unwrap();
    let two = two.unwrap();

    if let Some(bg) = two.bg {
        one.bg = Some(bg);
    }

    if let Some(fg) = two.fg {
        one.fg = Some(fg);
    }

    if let Some(s) = two.text_style {
        one.text_style = Some(s);
    }

    Some(one)
}
