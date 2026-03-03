use eframe::egui;
use sanedit_messages::redraw::{Color, Rgb, Style};

pub struct EguiStyle {
    pub fg: Option<egui::Color32>,
    pub bg: Option<egui::Color32>,
}

impl From<Style> for EguiStyle {
    fn from(style: Style) -> Self {
        let bg = style.bg.map(convert_color);
        let fg = style.fg.map(convert_color);

        EguiStyle { fg, bg }
    }
}

pub fn convert_color(color: Color) -> egui::Color32 {
    match color {
        Color::Black => egui::Color32::BLACK,
        Color::White => egui::Color32::WHITE,
        Color::Rgb(Rgb { red, green, blue }) => egui::Color32::from_rgb(red, green, blue),
    }
}
