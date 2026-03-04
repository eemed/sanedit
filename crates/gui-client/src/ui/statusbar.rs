use eframe::egui;
use sanedit_messages::redraw::{statusline::Statusline, Theme, ThemeField};

use crate::ui::style::EguiStyle;

pub struct StatusBar {
    pub statusline: Statusline,
    pub height: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            statusline: Statusline::default(),
            height: 24.0,
        }
    }

    /// Draw the status bar at the top
    pub fn show(&self, ctx: &egui::Context, theme: &Theme) {
        let EguiStyle { fg, bg } = theme.get(ThemeField::Statusline).into();

        egui::TopBottomPanel::top("status_bar")
            .resizable(false)
            .frame(egui::Frame {
                fill: bg,
                inner_margin: egui::Margin::same(4.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.statusline.left).color(fg));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(&self.statusline.right).color(fg));
                    });
                });
            });
    }
}
