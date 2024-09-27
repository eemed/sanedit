pub type TextStyle = u8;

pub const BOLD: u8 = 1 << 0;
pub const UNDERLINE: u8 = 1 << 1;
pub const ITALIC: u8 = 1 << 2;

pub fn from_str(string: &str) -> TextStyle {
    let mut style = 0u8;

    for s in string.split(',') {
        match s {
            "bold" => style |= BOLD,
            "underline" => style |= UNDERLINE,
            "italic" => style |= ITALIC,
            _ => {}
        }
    }
    style
}
