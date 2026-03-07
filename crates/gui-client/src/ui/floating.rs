use std::{cmp::max, sync::Arc};

use eframe::egui::{self, text::LayoutJob, FontId, TextFormat};
use sanedit_messages::redraw::{
    Cell, Popup, PopupKind, PopupMessageText, Size, Style, Theme, ThemeField,
};

use crate::ui::style::EguiStyle;

pub struct Floating {
    pub font_size: f32,
}

impl Floating {
    pub fn new(font_size: f32) -> Self {
        Self { font_size }
    }

    fn font_id(&self, ui: &mut egui::Ui) -> egui::FontId {
        let mut font = egui::TextStyle::Body.resolve(ui.style());
        font.size = self.font_size;
        font
    }

    fn format_item(lines: &[Vec<Cell>], font_id: FontId, max_width: f32) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.wrap.max_width = max_width;

        // TODO optimize?
        for line in lines {
            if !job.is_empty() {
                job.append(
                    "\n",
                    0.0,
                    TextFormat {
                        font_id: font_id.clone(),
                        ..Default::default()
                    },
                );
            }

            for cell in line {
                let style = EguiStyle::from(cell.style);
                job.append(
                    &cell.text,
                    0.0,
                    TextFormat {
                        font_id: font_id.clone(),
                        color: style.fg,
                        ..Default::default()
                    },
                );
            }
        }

        job
    }

    pub fn draw_popups(&self, ui: &mut egui::Ui, popup: &Popup) -> Vec<Arc<egui::Galley>> {
        let font_id = self.font_id(ui);
        let mut galleys = vec![];

        for message in &popup.messages {
            match &message.text {
                PopupMessageText::Formatted(cells) => {
                    let text =
                        Self::format_item(cells.as_slice(), font_id.clone(), ui.available_width());
                    let galley = ui.fonts(|f| f.layout_job(text));
                    galleys.push(galley);
                    // let (rect, _response) =
                    //     ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                    // ui.painter().galley(rect.min, galley);
                }
                PopupMessageText::Plain(text) => {
                    let mut job = LayoutJob::default();
                    job.append(
                        text,
                        0.0,
                        TextFormat {
                            font_id: font_id.clone(),
                            ..Default::default()
                        },
                    );
                    let galley = ui.fonts(|f| f.layout_job(job));
                    galleys.push(galley);
                }
            }
        }

        galleys
    }

    fn popup_position_from_cell(
        screen: egui::Rect,
        cell_pos: egui::Pos2,
        popup_size: egui::Vec2,
        cell_size: egui::Vec2,
        margin: f32,
    ) -> egui::Pos2 {
        let space_above = cell_pos.y - screen.top();
        let space_below = screen.bottom() - cell_pos.y - cell_size.y;

        let y = if space_above >= popup_size.y + margin {
            (cell_pos.y - popup_size.y - margin).max(0.0) // above
        } else if space_below >= popup_size.y + margin {
            cell_pos.y + cell_size.y + margin // below
        } else {
            margin
        };

        // let min = screen.left() + margin;
        let width = popup_size.x + margin + margin + 8.0;
        let mut x = cell_pos.x;
        if x + width > screen.width() {
            let diff = x + width - screen.width();
            x -= diff;
        }

        egui::Pos2::new(x, y)
    }

    pub fn show(
        &self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        popup: &Popup,
        theme: &Theme,
        cell_size: egui::Vec2,
    ) {
        let screen_rect = ctx.input(|i| i.screen_rect());
        let title_style = EguiStyle::from(theme.get(ThemeField::PopupDefault));
        let grid_offset = ui.min_rect().min;
        let cell_pos = egui::Pos2::new(
            grid_offset.x + popup.point.x as f32 * cell_size.x,
            grid_offset.y + popup.point.y as f32 * cell_size.y,
        );
        let popups = self.draw_popups(ui, popup);
        let popup_size = popups.iter().fold(egui::Vec2::ZERO, |mut acc, galley| {
            acc.x = acc.x.max(galley.size().x);
            acc.y += galley.size().y;
            acc
        });
        let pos = Self::popup_position_from_cell(screen_rect, cell_pos, popup_size, cell_size, 2.0);

        egui::Area::new("popup_area")
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .inner_margin(egui::Margin::same(4.0))
                    .fill(title_style.bg)
                    .rounding(egui::Rounding::same(2.0))
                    .stroke(egui::Stroke::new(1.0, title_style.fg))
                    .shadow(egui::epaint::Shadow::small_light())
                    .show(ui, |ui| {
                        for galley in popups {
                            let (rect, _) =
                                ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                            ui.painter().galley(rect.min, galley);
                        }
                    });
            });
    }
}
