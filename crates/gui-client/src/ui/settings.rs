use eframe::egui::{self, text::LayoutJob, FontId, TextFormat, Ui};
use sanedit_messages::redraw::{choice::Choice, prompt::Prompt, Theme, ThemeField};

use crate::ui::style::EguiStyle;

pub struct Settings {
    pub editor_font: String,
    pub editor_font_size: f32,
    pub ui_font: String,
    pub ui_font_size: f32,
    pub open: bool,
}

impl Settings {
    pub fn new() -> Self {
        let editor_font_size = 12_f32 * 1.35;
        Self {
            ui_font: "".into(),
            ui_font_size: editor_font_size - 1.0,
            editor_font: "".into(),
            editor_font_size,
            open: false,
        }
    }

    fn ui_font_id(&self, ui: &mut Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.ui_font_size;
        font
    }

    fn draw(&mut self, ui: &mut Ui, theme: &Theme) {
        let title_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayTitle));
        let msg_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayMessage));

        let ui_font = self.ui_font_id(ui);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Settings")
                    .font(ui_font.clone())
                    .color(title_style.fg),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("X").clicked() {
                    self.open = false;
                }
            });
        });

        egui::CollapsingHeader::new("Fonts")
            .default_open(true)
            .show(ui, |ui| {
                // --- Editor Font ---
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Editor Font:")
                            .font(ui_font.clone())
                            .color(msg_style.fg),
                    );
                    egui::ComboBox::from_id_salt("editor_font")
                        .selected_text(&self.editor_font)
                        .show_ui(ui, |ui| {
                            for font_name in
                                &["Monospace", "JetBrains Mono", "ComicMono", "Courier"]
                            {
                                ui.selectable_value(
                                    &mut self.editor_font,
                                    font_name.to_string(),
                                    *font_name,
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Editor Font Size:")
                            .font(ui_font.clone())
                            .color(msg_style.fg),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.editor_font_size)
                            .range(8.0..=32.0)
                            .speed(1.0),
                    );
                });

                // --- UI Font ---
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("UI Font:")
                            .font(ui_font.clone())
                            .color(msg_style.fg),
                    );
                    egui::ComboBox::from_id_salt("ui_font")
                        .selected_text(&self.ui_font)
                        .show_ui(ui, |ui| {
                            for font_name in &["Body", "Monospace", "Sans", "Serif"] {
                                ui.selectable_value(
                                    &mut self.ui_font,
                                    font_name.to_string(),
                                    *font_name,
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("UI Font Size:")
                            .font(ui_font.clone())
                            .color(msg_style.fg),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.ui_font_size)
                            .range(8.0..=32.0)
                            .speed(1.0),
                    );
                });
            });
    }

    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        if !self.open {
            return;
        }

        let screen_rect = ctx.input(|i| i.content_rect());

        let width = screen_rect.width() * 0.5;
        let height = 340.0;
        let size = egui::vec2(width, height);

        let pos = egui::pos2(
            screen_rect.center().x - width / 2.0,
            screen_rect.top() + 80.0,
        );

        let title_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayTitle));

        egui::Area::new("settings_area".into())
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::Frame::default()
                        .inner_margin(egui::Margin::same(4))
                        .fill(title_style.bg)
                        .corner_radius(4)
                        .stroke(egui::Stroke::new(1.0, title_style.fg))
                        // .shadow(egui::epaint::Shadow::())
                        .show(ui, |ui| self.draw(ui, theme));
                });
            });
    }
}
