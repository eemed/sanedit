use eframe::egui;

#[derive(Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: egui::Color32,
    pub bg: egui::Color32,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: egui::Color32::WHITE,
            bg: egui::Color32::TRANSPARENT,
        }
    }
}
