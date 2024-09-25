use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    text_style::{self, TextStyle},
    Color, HexStringError,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Style {
    pub text_style: Option<TextStyle>,
    pub bg: Option<Color>,
    pub fg: Option<Color>,
}

impl Style {
    // pub fn invert(&mut self) {
    //     mem::swap(&mut self.bg, &mut self.fg);
    // }

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

    /// Parse a style from a string of form "bg,fg,text_styles[,tex_styles]"
    pub fn from_str(string: &str) -> Result<Style, StyleError> {
        if string.is_empty() {
            return Ok(Style {
                text_style: None,
                bg: None,
                fg: None,
            });
        }

        let splits: Vec<&str> = string.splitn(3, ",").collect();
        if splits.len() < 3 {
            return Err(StyleError::Split);
        }

        let bg = Color::from_str(splits[0]).ok();
        let fg = Color::from_str(splits[1]).ok();
        let text_style = text_style::from_str(splits[2]);

        Ok(Style {
            bg,
            fg,
            text_style: Some(text_style),
        })
    }
}

#[derive(Error, Debug)]
pub enum StyleError {
    #[error("Failed to parse color")]
    ColorError(#[from] HexStringError),

    #[error("Too few splits, the format is bg,fg,attr[,attr]*")]
    Split,
}
