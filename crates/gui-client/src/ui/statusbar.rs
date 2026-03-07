use eframe::egui;
use sanedit_messages::redraw::{Theme, ThemeField, status::{Mode, Status}};

use crate::ui::style::EguiStyle;

pub struct StatusBar {
    pub height: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            height: 24.0,
        }
    }

    /// Draw the status bar at the top
    pub fn show(&self, ctx: &egui::Context, status: &Status, theme: &Theme) {
        let EguiStyle { fg, bg } = theme.get(ThemeField::Statusline).into();

        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
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
                        ui.label(egui::RichText::new(&format!("{}%", status.cursor_percentage)).color(fg));
                        if status.macro_recording {
                            ui.label(egui::RichText::new(" Recording macro ").color(fg));
                        }
                        ui.label(egui::RichText::new(&status.pressed_keys).color(fg));
                    });
                });
            });
    }
}
