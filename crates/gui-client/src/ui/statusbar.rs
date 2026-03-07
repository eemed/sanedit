use eframe::egui;
use sanedit_messages::redraw::{
    status::{Mode, Status},
    Theme, ThemeField,
};

use crate::ui::style::EguiStyle;

pub struct StatusBar {
    pub height: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { height: 24.0 }
    }

    /// Draw the status bar at the top
    pub fn show(&self, ctx: &egui::Context, status: &Status, theme: &Theme) {
        let EguiStyle { fg, bg } = theme.get(ThemeField::Statusline).into();

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
                ui.horizontal(|ui| {
                    let text = match status.mode {
                        Mode::Normal => "Normal",
                        Mode::Insert => "Insert",
                        Mode::Select => "Select",
                    };
                    ui.label(egui::RichText::new(text).color(fg));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(&status.language).color(fg));
                        ui.separator();
                        ui.label(
                            egui::RichText::new(&format!(
                                "{}: {}",
                                &status.indent_kind, status.indent_amount
                            ))
                            .color(fg),
                        );
                        ui.separator();
                        ui.label(egui::RichText::new(&status.end_of_line).color(fg));
                        ui.separator();
                        ui.label(
                            egui::RichText::new(&format!("{}%", status.cursor_percentage))
                                .color(fg),
                        );
                        if status.macro_recording {
                            ui.separator();
                            ui.label(egui::RichText::new(" Recording macro ").color(fg));
                        }
                        if !status.pressed_keys.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new(&status.pressed_keys).color(fg));
                        }
                    });
                });
            });
    }
}
