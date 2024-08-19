use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
#[derive(Serialize, Deserialize)]
pub struct TextStyle: u8 {
    const BOLD      = 0b00000001;
    const UNDERLINE = 0b00000010;
    const ITALIC    = 0b00000100;
}}

impl TextStyle {
    pub fn from_str(string: &str) -> TextStyle {
        let mut style = TextStyle::empty();

        for s in string.split(",") {
            match s {
                "bold" => style |= TextStyle::BOLD,
                "underline" => style |= TextStyle::UNDERLINE,
                "italic" => style |= TextStyle::ITALIC,
                _ => {}
            }
        }
        style
    }
}
