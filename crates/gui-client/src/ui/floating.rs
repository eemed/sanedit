use eframe::egui;
use sanedit_messages::redraw::{Popup, Theme, ThemeField};

use crate::ui::style::EguiStyle;

pub struct Floating {
    pub popup: Option<Popup>,
    pub font_size: f32,
}

impl Floating {
    pub fn new(font_size: f32) -> Self {
        Self {
            popup: None,
            font_size,
        }
    }

    fn font_id(&self, ui: &mut egui::Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.font_size;
        font
    }

    pub fn draw(&mut self, ui: &mut egui::Ui, theme: &Theme, width: f32) {
        let font_id = self.font_id(ui);
        ui.set_width(width);
        ui.add_space(14.0);
        // ui.horizontal(|ui| {
        //     ui.add_space(12.0);

        //     ui.label(
        //         egui::RichText::new("🔎")
        //             .size(self.font_size * 1.2)
        //             .color(title_style.fg),
        //     );

        //     ui.add_space(8.0);

        //     if prompt.input.is_empty() {
        //         ui.label(
        //             egui::RichText::new(&prompt.message)
        //                 .size(self.font_size)
        //                 .color(msg_style.fg),
        //         );
        //     } else {
        //         ui.label(
        //             egui::RichText::new(&prompt.input)
        //                 .color(input_style.fg)
        //                 .size(self.font_size),
        //         );
        //     }
        // });

        // ui.add_space(14.0);

    }

    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        if self.popup.is_none() {
            return;
        }
        let screen_rect = ctx.input(|i| i.screen_rect());

        let width = screen_rect.width() * 0.5;
        let height = 340.0;
        let size = egui::vec2(width, height);

        let pos = egui::pos2(
            screen_rect.center().x - width / 2.0,
            screen_rect.top() + 80.0,
        );

        let title_style = EguiStyle::from(theme.get(ThemeField::PopupDefault));
        egui::Area::new("popup_area")
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::Frame::default()
                        .inner_margin(egui::Margin::same(4.0))
                        .fill(title_style.bg)
                        .rounding(egui::Rounding::same(6.0))
                        .stroke(egui::Stroke::new(1.0, title_style.fg))
                        .shadow(egui::epaint::Shadow::small_dark())
                        .show(ui, |ui| self.draw(ui, theme, width));
                });
            });
    }
}
