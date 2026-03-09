use eframe::egui::{self, text::LayoutJob, FontId, TextFormat, Ui};
use sanedit_messages::redraw::{choice::Choice, prompt::Prompt, Theme, ThemeField};

use crate::ui::style::EguiStyle;

use super::settings::Settings;

pub struct Select {
}

impl Select {
    pub fn new() -> Self {
        Self {
        }
    }

    fn format_item(
        choice: &Choice,
        font_id: FontId,
        normal: &EguiStyle,
        hls: &EguiStyle,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();

        let mut at = 0;
        for hl in &choice.matches {
            if hl.start != 0 {
                job.append(
                    &choice.text[at..hl.start],
                    0.0,
                    TextFormat {
                        font_id: font_id.clone(),
                        color: normal.fg,
                        ..Default::default()
                    },
                );
            }

            job.append(
                &choice.text[hl.start..hl.end],
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: hls.fg,
                    ..Default::default()
                },
            );

            at = hl.end;
        }

        if at != choice.text.len() {
            job.append(
                &choice.text[at..],
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: normal.fg,
                    ..Default::default()
                },
            );
        }

        job
    }

    pub fn draw(&self, ui: &mut Ui, prompt: &Prompt, settings: &Settings, theme: &Theme, width: f32) {
        let font_id = settings.ui_font_id(ui);
        // Styles
        let sel_style = EguiStyle::from(theme.get(ThemeField::PromptCompletionSelected));
        let compl_style = EguiStyle::from(theme.get(ThemeField::PromptCompletion));
        let match_style = EguiStyle::from(theme.get(ThemeField::PromptCompletionMatch));
        let descr_style = EguiStyle::from(theme.get(ThemeField::PromptCompletionDescription));
        let sel_match_style = EguiStyle::from(theme.get(ThemeField::PromptCompletionSelectedMatch));
        let sel_descr_style =
            EguiStyle::from(theme.get(ThemeField::PromptCompletionSelectedDescription));
        let input_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayInput));
        let title_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayTitle));
        let msg_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayMessage));

        // UI
        ui.set_width(width);
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);

            ui.label(
                egui::RichText::new("🔎")
                    .font(font_id.clone())
                    .color(title_style.fg),
            );

            ui.add_space(8.0);

            if prompt.input.is_empty() {
                ui.label(
                    egui::RichText::new(&prompt.message)
                    .font(font_id.clone())
                        .color(msg_style.fg),
                );
            } else {
                ui.label(
                    egui::RichText::new(&prompt.input)
                        .color(input_style.fg)
                    .font(font_id.clone()),
                );
            }
        });

        ui.add_space(14.0);

        if !prompt.options.is_empty() {
            ui.separator();

            for (i, item) in prompt.options.iter().enumerate() {
                let selected = Some(i) == prompt.selected;
                let item_style = if selected { &sel_style } else { &compl_style };
                let descr_style = if selected {
                    &sel_descr_style
                } else {
                    &descr_style
                };
                let mat_style = if selected {
                    &sel_match_style
                } else {
                    &match_style
                };

                let (rect, _response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 30.0),
                    egui::Sense::hover(),
                );

                const RIGHT_WIDTH: f32 = 120.0;
                const PADDING: f32 = 14.0;

                ui.painter().rect_filled(rect, 4.0, item_style.bg);

                // split the row into two columns
                let left_rect = if item.description.is_empty() {
                    rect
                } else {
                    egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.max.x - RIGHT_WIDTH, rect.max.y),
                    )
                };

                let left_painter = ui.painter().with_clip_rect(left_rect);
                let left_text = Self::format_item(item, font_id.clone(), item_style, mat_style);
                let galley = ui.fonts_mut(|f| f.layout_job(left_text));
                left_painter.galley(
                    rect.left_center() + egui::vec2(PADDING, -galley.size().y / 2.0),
                    galley,
                    item_style.bg,
                );

                if !item.description.is_empty() {
                    let right_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.max.x - RIGHT_WIDTH, rect.min.y),
                        rect.max,
                    );
                    let right_painter = ui.painter().with_clip_rect(right_rect);

                    right_painter.text(
                        right_rect.right_center() - egui::vec2(PADDING, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        &item.description,
                        font_id.clone(),
                        descr_style.fg,
                    );
                }
            }
        }
    }

    pub fn show(&self, ctx: &egui::Context, prompt: &Prompt, settings: &Settings, theme: &Theme) {
        let screen_rect = ctx.input(|i| i.content_rect());

        let width = screen_rect.width() * 0.5;
        let height = 340.0;
        let size = egui::vec2(width, height);

        let pos = egui::pos2(
            screen_rect.center().x - width / 2.0,
            screen_rect.top() + 80.0,
        );

        let title_style = EguiStyle::from(theme.get(ThemeField::PromptOverlayTitle));

        egui::Area::new("prompt_area".into())
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
                        .show(ui, |ui| self.draw(ui, prompt, settings, theme, width));
                });
            });
    }
}
