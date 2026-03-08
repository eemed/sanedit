use eframe::egui;
use sanedit_messages::redraw::{
    items::{ItemKind, Items},
    status::{Mode, Status},
    Theme, ThemeField,
};

use crate::ui::style::EguiStyle;

pub struct Filetree {
    pub max_width: f32,
    pub font_size: f32,
}

impl Filetree {
    pub fn new(font_size: f32, max_width: f32) -> Self {
        Self {
            max_width,
            font_size,
        }
    }

    fn font_id(&self, ui: &mut egui::Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.font_size;
        font
    }

    pub fn show(&self, ctx: &egui::Context, items: &Items, theme: &Theme) {
        let EguiStyle { bg, .. } = theme.get(ThemeField::FiletreeDefault).into();

        egui::SidePanel::left("file_tree_panel")
            .resizable(true)
            .frame(egui::Frame::default().fill(bg))
            .default_width(200.0)
            .max_width(self.max_width)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.draw(ui, items, theme);
                })
            });
    }

    fn draw(&self, ui: &mut egui::Ui, items: &Items, theme: &Theme) {
        const LEVEL_INDENT: f32 = 20.0;
        let row_height = self.font_size + 6.0;

        let style = EguiStyle::from(theme.get(ThemeField::FiletreeDefault));
        let file_style = EguiStyle::from(theme.get(ThemeField::FiletreeFile));
        let error_style = EguiStyle::from(theme.get(ThemeField::FiletreeError));
        let dir_style = EguiStyle::from(theme.get(ThemeField::FiletreeDir));
        let marker_style = EguiStyle::from(theme.get(ThemeField::FiletreeMarkers));
        let symlink_style = EguiStyle::from(theme.get(ThemeField::FiletreeSymlink));

        let sel_style = EguiStyle::from(theme.get(ThemeField::FiletreeSelected));
        let sel_file_style = EguiStyle::from(theme.get(ThemeField::FiletreeSelectedFile));
        let sel_error_style = EguiStyle::from(theme.get(ThemeField::FiletreeSelectedError));
        let sel_dir_style = EguiStyle::from(theme.get(ThemeField::FiletreeSelectedDir));
        let sel_marker_style = EguiStyle::from(theme.get(ThemeField::FiletreeSelectedMarkers));
        let sel_symlink_style = EguiStyle::from(theme.get(ThemeField::FiletreeSelectedSymlink));
        let font_id = self.font_id(ui);

        ui.horizontal(|ui| {
            egui::Frame::none().rounding(4.0).show(ui, |ui| {
                ui.set_width(ui.available_width());

                let text = egui::RichText::new("File browser")
                    .heading()
                    .font(font_id.clone());
                ui.label(text);
            })
        });

        for (i, item) in items.items.iter().enumerate() {
            let is_selected = items.selected == i;
            let row_style = if is_selected { &sel_style } else { &style };

            ui.horizontal(|ui| {
                egui::Frame::none()
                    .rounding(4.0)
                    .fill(row_style.bg)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width()- 2.0);
                        ui.set_height(row_height);
                        ui.add_space(8.0 + item.level as f32 * LEVEL_INDENT);

                        let style = match (
                            item.is_symlink,
                            item.is_readable,
                            item.kind == ItemKind::Item,
                            is_selected,
                        ) {
                            (_, false, _, true) => &sel_error_style,
                            (_, false, _, false) => &error_style,
                            (true, _, _, true) => &sel_symlink_style,
                            (true, _, _, false) => &symlink_style,
                            (_, _, true, true) => &sel_file_style,
                            (_, _, true, false) => &file_style,
                            (_, _, false, true) => &sel_dir_style,
                            (_, _, false, false) => &dir_style,
                        };
                        let mark_style = if is_selected {
                            &sel_marker_style
                        } else {
                            &marker_style
                        };

                        match item.kind {
                            ItemKind::Group { expanded } => {
                                if expanded {
                                    let text = egui::RichText::new("-")
                                        .background_color(mark_style.bg)
                                        .color(mark_style.fg)
                                        .font(font_id.clone());
                                    ui.label(text);
                                } else {
                                    let text = egui::RichText::new("+")
                                        .background_color(mark_style.bg)
                                        .color(mark_style.fg)
                                        .font(font_id.clone());
                                    ui.label(text);
                                }

                                let text = egui::RichText::new(&item.name)
                                    .background_color(style.bg)
                                    .color(style.fg)
                                    .font(font_id.clone());
                                ui.label(text);
                            }
                            ItemKind::Item => {
                                let text = egui::RichText::new("#")
                                    .color(mark_style.fg)
                                    .background_color(mark_style.bg)
                                    .font(font_id.clone());
                                ui.label(text);
                                let text = egui::RichText::new(&item.name)
                                    .background_color(style.bg)
                                    .color(style.fg)
                                    .font(font_id.clone());
                                ui.label(text);
                            }
                        }

                        ui.add_space(8.0);
                    })
            });
        }
    }
}
