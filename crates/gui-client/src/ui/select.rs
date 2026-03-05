use eframe::egui::{self, Ui};
use sanedit_messages::redraw::{prompt::Prompt, Theme, ThemeField};

use crate::ui::style::EguiStyle;

pub struct Select {
    pub prompt: Option<Prompt>,
    pub font_size: f32,
}

impl Select {
    pub fn new(font_size: f32) -> Self {
        Self {
            prompt: None,
            font_size,
        }
    }

    fn font_id(&self, ui: &mut Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.font_size;
        font
    }

    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        if let Some(ref prompt) = self.prompt {
            let screen_rect = ctx.input(|i| i.screen_rect());

            let width = screen_rect.width() * 0.5;
            let height = 340.0;
            let size = egui::vec2(width, height);

            let pos = egui::pos2(
                screen_rect.center().x - width / 2.0,
                screen_rect.top() + 80.0,
            );

            egui::Area::new("prompt_area")
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    ui.allocate_ui_with_layout(
                        size,
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::default()
                                .fill(egui::Color32::from_rgb(35, 35, 38))
                                .rounding(egui::Rounding::same(6.0)) // less rounded
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70)))
                                .shadow(egui::epaint::Shadow::small_dark())
                                .show(ui, |ui| {
                                    // Styles
                                    let sel_style = EguiStyle::from(
                                        theme.get(ThemeField::PromptCompletionSelected),
                                    );
                                    let compl_style =
                                        EguiStyle::from(theme.get(ThemeField::PromptCompletion));
                                    let default_style =
                                        EguiStyle::from(theme.get(ThemeField::PromptDefault));
                                    let match_style = EguiStyle::from(
                                        theme.get(ThemeField::PromptCompletionMatch),
                                    );
                                    let descr_style = EguiStyle::from(
                                        theme.get(ThemeField::PromptCompletionDescription),
                                    );
                                    let sel_match_style = EguiStyle::from(
                                        theme.get(ThemeField::PromptCompletionSelectedMatch),
                                    );
                                    let sel_descr_style = EguiStyle::from(
                                        theme.get(ThemeField::PromptCompletionSelectedDescription),
                                    );
                                    let input_style =
                                        EguiStyle::from(theme.get(ThemeField::PromptOverlayInput));
                                    let title_style =
                                        EguiStyle::from(theme.get(ThemeField::PromptOverlayTitle));
                                    let msg_style = EguiStyle::from(
                                        theme.get(ThemeField::PromptOverlayMessage),
                                    );

                                    // UI
                                    ui.set_width(width);
                                    ui.add_space(14.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(12.0);

                                        ui.label(
                                            egui::RichText::new("🔎")
                                                .size(self.font_size * 1.2)
                                                .color(egui::Color32::from_gray(180)),
                                        );

                                        ui.add_space(8.0);

                                        if prompt.input.is_empty() {
                                            ui.label(
                                                egui::RichText::new(&prompt.message)
                                                    .size(self.font_size)
                                                    .weak(),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new(&prompt.input)
                                                    .size(self.font_size),
                                            );
                                        }
                                    });

                                    ui.add_space(14.0);

                                    let font_id = self.font_id(ui);
                                    if !prompt.options.is_empty() {
                                        ui.separator();

                                        for (i, item) in prompt.options.iter().enumerate() {
                                            let selected = Some(i) == prompt.selected;

                                            let item_style =
                                                if selected { &sel_style } else { &compl_style };

                                            let (rect, _response) = ui.allocate_exact_size(
                                                egui::vec2(ui.available_width(), 30.0),
                                                egui::Sense::click(),
                                            );

                                            ui.painter().rect_filled(rect, 0.0, item_style.bg);

                                            ui.painter().text(
                                                rect.left_center() + egui::vec2(14.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                &item.text,
                                                font_id.clone(),
                                                item_style.fg,
                                            );
                                        }
                                    }
                                });
                        },
                    );
                });
        }
    }
}
