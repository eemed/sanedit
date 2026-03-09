use eframe::egui::{self};
use sanedit_messages::redraw::{window::Window, Cell, Cursor, Point, Size, Theme, ThemeField};

use crate::ui::style::EguiStyle;

use super::settings::Settings;

pub struct CharGrid {
    pub window: Window,
    pub cell_size: Option<(egui::FontId, egui::Vec2)>,
}

impl CharGrid {
    pub fn new() -> Self {
        Self {
            window: Window::default(),
            cell_size: None,
        }
    }

    fn cell_size(&mut self, ui: &mut egui::Ui, font_id: egui::FontId) -> egui::Vec2 {
        fn compute_cell_size(ui: &mut egui::Ui, font_id: egui::FontId) -> egui::Vec2 {
            ui.fonts_mut(|f| {
                let row_height = f.row_height(&font_id);
                let glyph_width = f.glyph_width(&font_id, 'M');
                egui::vec2(glyph_width.ceil(), row_height.ceil())
            })
        }

        if let Some((id, size)) = &self.cell_size {
            if &font_id == id {
                return *size;
            }
        }

        let size = compute_cell_size(ui, font_id.clone());
        self.cell_size = Some((font_id, size));
        size
    }

    pub fn size(&mut self, ui: &mut egui::Ui, settings: &Settings) -> Size {
        let font_id = settings.editor_font_id(ui);
        let cell_size = self.cell_size(ui, font_id);
        let available = ui.available_size();
        let cols = (available.x / cell_size.x).floor() as usize;
        let rows = (available.y / cell_size.y).floor() as usize;

        Size {
            width: cols,
            height: rows,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        settings: &Settings,
        theme: &Theme,
    ) {
        fn draw_cell(
            painter: &egui::Painter,
            ui: &mut egui::Ui,
            pos: egui::Pos2,
            cell: &Cell,
            font_id: egui::FontId,
            fg: egui::Color32,
        ) {
            painter.text(pos, egui::Align2::LEFT_TOP, &cell.text, font_id, fg);
            // let mut job = LayoutJob::default();

            // job.append(
            //     &cell.text,
            //     0.0,
            //     egui::TextFormat {
            //         font_id,
            //         color: fg,
            //         valign: egui::Align::Center,
            //         ..Default::default()
            //     },
            // );

            // let galley = ui.fonts_mut(|f| f.layout_job(job));

            // painter.galley(pos, galley, fg);
        }

        let default = EguiStyle::from(theme.get(ThemeField::Default));
        let total_rows = self.window.height();
        if total_rows == 0 {
            return;
        }
        let total_cols = self.window.width();
        let font_id = settings.editor_font_id(ui);
        let cell_size = self.cell_size(ui, font_id.clone());
        let available = ui.available_size();
        let visible_cols = total_cols.min((available.x / cell_size.x).floor() as usize);
        let visible_rows = total_rows.min((available.y / cell_size.y).floor() as usize);
        let desired_size = egui::vec2(
            visible_cols as f32 * cell_size.x,
            visible_rows as f32 * cell_size.y,
        );

        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 0.0, default.bg);

        // let clip_fix = egui::vec2(0.0, 2.0);
        let clip_fix = egui::vec2(0.0, 0.0);

        for (row, col, cell) in self.window.used() {
            let mut pos = egui::pos2(
                rect.left() + col as f32 * cell_size.x,
                rect.top() + row as f32 * cell_size.y,
            );

            let style = EguiStyle::from(cell.style);
            painter.rect_filled(egui::Rect::from_min_size(pos, cell_size), 0.0, style.bg);

            pos += clip_fix;
            draw_cell(&painter, ui, pos, &cell, font_id.clone(), style.fg);
        }

        let cursor = EguiStyle::from(theme.get(ThemeField::Cursor));

        if let Some(Cursor {
            bg,
            fg,
            shape,
            point: Point { x, y },
        }) = self.window.cursor
        {
            let mut cursor_pos = egui::pos2(
                rect.left() + x as f32 * cell_size.x,
                rect.top() + y as f32 * cell_size.y,
            );

            // Draw block cursor
            painter.rect_filled(
                egui::Rect::from_min_size(cursor_pos, cell_size),
                0.0,
                cursor.bg,
            );

            let cell = self.window.at(y, x);

            cursor_pos += clip_fix;

            draw_cell(&painter, ui, cursor_pos, &cell, font_id.clone(), cursor.fg);
        }
    }
}
