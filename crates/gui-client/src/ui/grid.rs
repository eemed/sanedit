use crate::ui::{
    cell::Cell,
    style::{convert_color, EguiStyle},
};
use eframe::egui::{self, Color32};
use sanedit_messages::redraw::{
    window::Window, Cursor, CursorShape, Point, Size, Theme, ThemeField,
};

pub struct CharGrid {
    pub window: Window,
    pub font_size: f32,

    cell_size: Option<egui::Vec2>,
}

impl CharGrid {
    pub fn new(font_size: f32) -> Self {
        Self {
            window: Window::default(),
            font_size,
            cell_size: None,
        }
    }

    fn font_id(&self) -> egui::FontId {
        egui::FontId::monospace(self.font_size)
    }

    fn compute_cell_size(&self, ui: &egui::Ui) -> egui::Vec2 {
        let font_id = self.font_id();

        ui.fonts(|f| {
            let row_height = f.row_height(&font_id);
            let glyph_width = f.glyph_width(&font_id, 'W');
            egui::vec2(glyph_width.ceil(), row_height.ceil())
        })
    }

    fn cell_size(&mut self, ui: &egui::Ui) -> egui::Vec2 {
        if let Some(size) = self.cell_size {
            size
        } else {
            let size = self.compute_cell_size(ui);
            self.cell_size = Some(size);
            size
        }
    }

    pub fn invalidate_layout(&mut self) {
        self.cell_size = None;
    }

    pub fn size(&self, ui: &egui::Ui) -> Size {
        let cell_size = self.compute_cell_size(ui);
        let available = ui.available_size();
        let cols = (available.x / cell_size.x).floor() as usize;
        let rows = (available.y / cell_size.y).floor() as usize;

        Size {
            width: cols,
            height: rows,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let total_rows = self.window.height();
        if total_rows == 0 {
            return;
        }
        let total_cols = self.window.width();

        let cell_size = self.cell_size(ui);
        let font_id = self.font_id();

        let available = ui.available_size();
        let visible_cols = total_cols.min((available.x / cell_size.x).floor() as usize);
        let visible_rows = total_rows.min((available.y / cell_size.y).floor() as usize);

        let desired_size = egui::vec2(
            visible_cols as f32 * cell_size.x,
            visible_rows as f32 * cell_size.y,
        );

        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        let default = EguiStyle::from(theme.get(ThemeField::Default));

        painter.rect_filled(rect, 0.0, default.bg);

        for (row, col, cell) in self.window.used() {
            let mut pos = egui::pos2(
                rect.left() + col as f32 * cell_size.x,
                rect.top() + row as f32 * cell_size.y,
            );

            let style = EguiStyle::from(cell.style);

            if style.bg != egui::Color32::TRANSPARENT {
                painter.rect_filled(egui::Rect::from_min_size(pos, cell_size), 0.0, style.bg);
            }

            pos.y += cell_size.y;
            painter.text(
                pos,
                egui::Align2::LEFT_BOTTOM,
                cell.text.clone(),
                font_id.clone(),
                style.fg,
            );
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

            cursor_pos.y += cell_size.y;
            painter.text(
                cursor_pos,
                egui::Align2::LEFT_BOTTOM,
                cell.text.clone(),
                font_id.clone(),
                cursor.fg,
            );
        }
    }
}
