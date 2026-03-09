use std::sync::Arc;

use eframe::egui::{self, Ui};
use fontdb::Database;
use sanedit_messages::redraw::{Theme, ThemeField};

use crate::ui::style::EguiStyle;

pub struct Settings {
    editor_font: Option<(String, fontdb::ID)>,
    editor_filter: String,
    editor_font_size: f32,
    ui_font: Option<(String, fontdb::ID)>,
    ui_filter: String,
    ui_font_size: f32,

    fonts: fontdb::Database,
    available_proportional_fonts: Vec<(String, fontdb::ID)>,
    available_monospace_fonts: Vec<(String, fontdb::ID)>,
    pub open: bool,
}

impl Settings {
    pub fn new() -> Self {
        let editor_font_size = 16_f32;
        let mut fonts = fontdb::Database::new();
        fonts.load_system_fonts();
        let (available_proportional_fonts, available_monospace_fonts) = Self::font_families(&fonts);

        Self {
            ui_font: None,
            ui_filter: String::new(),
            ui_font_size: editor_font_size - 1.0,
            editor_font: None,
            editor_filter: String::new(),
            editor_font_size,
            fonts,
            available_proportional_fonts,
            available_monospace_fonts,
            open: false,
        }
    }

    fn load_font(&self, ctx: &egui::Context, id: fontdb::ID) {
        let mut fonts = egui::FontDefinitions::default();
        if let Some(font) = self.fonts.face(id) {
            if let Some((font_data, index)) =
                self.fonts.with_face_data(id, |data, i| (data.to_vec(), i))
            {
                if let Some((name, _)) = font.families.iter().next() {
                    fonts.font_data.insert(
                        name.clone(),
                        Arc::new(egui::FontData {
                            font: font_data.into(),
                            index,
                            tweak: Default::default(),
                        }),
                    );

                    let fam = if font.monospaced {
                        egui::FontFamily::Monospace
                    } else {
                        egui::FontFamily::Proportional
                    };

                    fonts
                        .families
                        .entry(fam)
                        .or_default()
                        .insert(0, name.clone());
                }
            }
        }

        ctx.set_fonts(fonts);
    }

    fn font_families(
        db: &fontdb::Database,
    ) -> (Vec<(String, fontdb::ID)>, Vec<(String, fontdb::ID)>) {
        let mut proportional = Vec::new();
        let mut monospace = Vec::new();

        for face in db.faces() {
            if face.monospaced {
                for (name, _) in &face.families {
                    monospace.push((name.clone(), face.id));
                }
            } else {
                for (name, _) in &face.families {
                    proportional.push((name.clone(), face.id));
                }
            }
        }

        proportional.sort();
        // proportional.dedup_by(|a, b| a.0 == b.0);

        monospace.sort();
        // monospace.dedup_by(|a, b| a.0 == b.0);

        (proportional, monospace)
    }

    pub fn ui_font_id(&self, ui: &mut Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.ui_font_size;
        font
    }

    pub fn editor_font_id(&self, ui: &mut Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Monospace.resolve(ui.style());
        font.size = self.editor_font_size;
        font
    }

    fn draw(&mut self, ui: &mut Ui, theme: &Theme) {
        let title_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayTitle));
        let msg_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayMessage));
        let item_style = EguiStyle::from(theme.get(ThemeField::Statusline));

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
                    ui.add(
                        egui::TextEdit::singleline(&mut self.editor_filter)
                            .desired_width(100.0)
                            .background_color(item_style.bg)
                            .text_color(item_style.fg)
                            .font(ui_font.clone())
                            .hint_text("Filter.."),
                    );

                    let name = self
                        .editor_font
                        .as_ref()
                        .map(|(a, _)| a.as_ref())
                        .unwrap_or("Default");
                    egui::ComboBox::from_id_salt("editor_font")
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            for (font_name, id) in &self.available_monospace_fonts {
                                if font_name.to_lowercase().contains(&self.editor_filter) {
                                    let btn = ui.selectable_value(
                                        &mut self.editor_font,
                                        Some((font_name.to_string(), *id)),
                                        egui::RichText::new(font_name)
                                            .font(ui_font.clone())
                                            .color(msg_style.fg),
                                    );

                                    if btn.clicked() {
                                        self.load_font(ui.ctx(), *id);
                                    }
                                }
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

                    ui.add(
                        egui::TextEdit::singleline(&mut self.ui_filter)
                            .desired_width(100.0)
                            .frame(true)
                            .background_color(item_style.bg)
                            .text_color(item_style.fg)
                            .font(ui_font.clone())
                            .hint_text("Filter.."),
                    );

                    let name = self
                        .ui_font
                        .as_ref()
                        .map(|(a, _)| a.as_ref())
                        .unwrap_or("Default");
                    egui::ComboBox::from_id_salt("ui_font")
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            for (font_name, id) in &self.available_proportional_fonts {
                                if font_name.to_lowercase().contains(&self.ui_filter) {
                                    let btn = ui.selectable_value(
                                        &mut self.ui_font,
                                        Some((font_name.to_string(), *id)),
                                        egui::RichText::new(font_name)
                                            .font(ui_font.clone())
                                            .color(msg_style.fg),
                                    );

                                    if btn.clicked() {
                                        self.load_font(ui.ctx(), *id);
                                    }
                                }
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
