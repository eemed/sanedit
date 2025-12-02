use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{text_style::TextStyle, Color, HexStringError};

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Default, PartialOrd, Ord, Hash,
)]
pub struct Style {
    pub text_style: Option<TextStyle>,
    pub bg: Option<Color>,
    pub fg: Option<Color>,
}

impl Style {
    /// use self as a base style and apply overrides
    pub fn merge(&mut self, style: &Style) {
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

#[derive(Error, Debug)]
pub enum StyleError {
    #[error("Failed to parse color")]
    ColorError(#[from] HexStringError),

    #[error("Too few splits, the format is bg,fg,attr[,attr]*")]
    Split,
}
