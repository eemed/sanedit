use eframe::egui;
use sanedit_messages::redraw::{
    status::{Mode, Status},
    Theme, ThemeField,
    items::Items,
};

use crate::ui::style::EguiStyle;

pub struct Filetree {
    pub max_width: f32,
    pub font_size: f32,
}

impl Filetree {
    pub fn new(font_size: f32, max_width: f32) -> Self {
        Self { max_width }
    }

    fn draw(&self, ui: &mut egui::Ui, items: &Items, theme: &Theme) {}

    pub fn show(&self, ctx: &egui::Context, status: &Filetree, theme: &Theme) {
        let EguiStyle { bg, .. } = theme.get(ThemeField::Statusline).into();

        egui::SidePanel::left("file_tree_panel")
            .resizable(true)
            .default_width(200.0)
            .max_width(self.max_width)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                })
            });
    }
}
