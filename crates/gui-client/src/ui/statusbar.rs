use eframe::egui::{self, Ui};
use sanedit_messages::redraw::{
    status::{Mode, Status},
    Theme, ThemeField,
};

use crate::ui::style::EguiStyle;

pub struct StatusBar {
    pub height: f32,
    pub font_size: f32,
}

impl StatusBar {
    pub fn new(font_size: f32, height: f32) -> Self {
        Self { height, font_size }
    }

    fn font_id(&self, ui: &mut Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.font_size;
        font
    }

    fn draw_line(&self, ui: &mut egui::Ui, status: &Status, theme: &Theme) {
        let font_id = self.font_id(ui);
        let EguiStyle { fg, .. } = theme.get(ThemeField::Statusline).into();
        let text = match status.mode {
            Mode::Normal => "Normal",
            Mode::Insert => "Insert",
            Mode::Select => "Select",
        };
        ui.label(egui::RichText::new(text).font(font_id.clone()).color(fg));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&status.language)
                    .font(font_id.clone())
                    .color(fg),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(&format!(
                    "{}: {}",
                    &status.indent_kind, status.indent_amount
                ))
                .font(font_id.clone())
                .color(fg),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(&status.end_of_line)
                    .font(font_id.clone())
                    .color(fg),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(&format!("{}%", status.cursor_percentage))
                    .font(font_id.clone())
                    .color(fg),
            );
            if status.macro_recording {
                ui.separator();
                ui.label(
                    egui::RichText::new(" Recording macro ")
                        .font(font_id.clone())
                        .color(fg),
                );
            }
            if !status.pressed_keys.is_empty() {
                ui.separator();
                ui.label(
                    egui::RichText::new(&status.pressed_keys)
                        .font(font_id.clone())
                        .color(fg),
                );
            }
        });
    }

    pub fn show(&self, ctx: &egui::Context, status: &Status, theme: &Theme) {
        let EguiStyle { bg, .. } = theme.get(ThemeField::Statusline).into();

        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .exact_height(self.height)
            .show_separator_line(false)
            .frame(egui::Frame {
                fill: bg,
                inner_margin: egui::Margin::same(4.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| self.draw_line(ui, status, theme));
            });
    }
}
