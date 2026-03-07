use eframe::egui;
use sanedit_messages::redraw::{Color, Rgb, Style};

pub struct EguiStyle {
    pub fg: egui::Color32,
    pub bg: egui::Color32,
}

impl Default for EguiStyle {
    fn default() -> Self {
        Self {
            fg: egui::Color32::WHITE,
            bg: egui::Color32::BLACK,
        }
    }
}

impl From<Style> for EguiStyle {
    fn from(style: Style) -> Self {
        let bg = style
            .bg
            .map(convert_color)
            .unwrap_or(egui::Color32::TRANSPARENT);
        let fg = style
            .fg
            .map(convert_color)
            .unwrap_or(egui::Color32::TRANSPARENT);

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
