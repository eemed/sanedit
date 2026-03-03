use crate::ui::cell::Cell;
use eframe::egui;
use sanedit_messages::redraw::{Cursor, CursorShape, Point, Size};

pub struct CharGrid {
    pub cells: Vec<Vec<Cell>>,
    pub scroll_row: usize,
    pub font_size: f32,

    pub cursor: Option<Cursor>,

    cell_size: Option<egui::Vec2>,
}

impl CharGrid {
    pub fn new(width: usize, height: usize, font_size: f32) -> Self {
        Self {
            cells: vec![vec![Cell::default(); width]; height],
            scroll_row: 0,
            font_size,
            cursor: None,
            cell_size: None,
        }
    }

    pub fn example_content(&mut self, rows: usize, cols: usize) {
        // Example content
        let code = r#"fn main() {
    println!("Hello, world!");
}"#;

        for (row, line) in code.lines().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if row >= rows || col >= cols {
                    continue;
                }

                let mut fg = egui::Color32::WHITE;

                // Highlight keywords
                let s: String = line.chars().collect();

                if s.starts_with("fn") && col < 2 {
                    fg = egui::Color32::from_rgb(0, 150, 255); // blue
                } else if s.contains("println!") {
                    let start = s.find("println!").unwrap();
                    if col >= start && col < start + 8 {
                        fg = egui::Color32::from_rgb(0, 200, 0); // green
                    }
                } else if s.contains('"') {
                    let first_quote = s.find('"').unwrap();
                    let last_quote = s.rfind('"').unwrap();
                    if col >= first_quote && col <= last_quote {
                        fg = egui::Color32::from_rgb(255, 200, 0); // yellow
                    }
                }

                self.cells[row][col] = Cell {
                    ch,
                    fg,
                    bg: egui::Color32::BLACK,
                };
            }
        }

        self.cursor = Some(Cursor {
            bg: None,
            fg: None,
            shape: CursorShape::Block(false),
            point: Point { x: 10, y: 1 },
        });
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

    pub fn show_grid(&mut self, ui: &mut egui::Ui) {
        let total_rows = self.cells.len();
        if total_rows == 0 {
            return;
        }
        let total_cols = self.cells[0].len();

        let cell_size = self.cell_size(ui);
        let font_id = self.font_id();

        let available = ui.available_size();
        let visible_cols = total_cols.min((available.x / cell_size.x).floor() as usize);
        let visible_rows = total_rows.min((available.y / cell_size.y).floor() as usize);

        self.scroll_row = self.scroll_row.min(total_rows.saturating_sub(visible_rows));

        let desired_size = egui::vec2(
            visible_cols as f32 * cell_size.x,
            visible_rows as f32 * cell_size.y,
        );

        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        // Wheel scrolling
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.scroll_delta.y);
            if scroll_delta != 0.0 {
                let delta_rows = (scroll_delta / cell_size.y).round() as isize;
                let new_row = self.scroll_row as isize - delta_rows;

                self.scroll_row =
                    new_row.clamp(0, total_rows.saturating_sub(visible_rows) as isize) as usize;
            }
        }

        let painter = ui.painter_at(rect);

        for row in 0..visible_rows {
            let doc_row = row + self.scroll_row;

            for col in 0..visible_cols {
                let cell = &self.cells[doc_row][col];

                let pos = egui::pos2(
                    rect.left() + col as f32 * cell_size.x,
                    rect.top() + row as f32 * cell_size.y + cell_size.y,
                );

                if cell.bg != egui::Color32::TRANSPARENT {
                    painter.rect_filled(egui::Rect::from_min_size(pos, cell_size), 0.0, cell.bg);
                }

                painter.text(
                    pos,
                    egui::Align2::LEFT_BOTTOM,
                    cell.ch,
                    font_id.clone(),
                    cell.fg,
                );
            }
        }

        if let Some(Cursor {
            bg,
            fg,
            shape,
            point: Point { x, y },
        }) = self.cursor
        {
            if y >= self.scroll_row && y < self.scroll_row + visible_rows {
                let screen_row = y - self.scroll_row;

                if x < visible_cols {
                    let cursor_pos = egui::pos2(
                        rect.left() + x as f32 * cell_size.x,
                        rect.top() + screen_row as f32 * cell_size.y,
                    );

                    // Draw block cursor
                    painter.rect_filled(
                        egui::Rect::from_min_size(cursor_pos, cell_size),
                        0.0,
                        egui::Color32::from_rgb(80, 80, 80), // gray highlight
                    );

                    // Optional: redraw character inverted
                    let cell = &self.cells[y][x];

                    painter.text(
                        cursor_pos,
                        egui::Align2::LEFT_TOP,
                        cell.ch,
                        font_id.clone(),
                        egui::Color32::WHITE,
                    );
                }
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.show_grid(ui);
    }
}
