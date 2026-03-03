use std::sync::Arc;

use eframe::egui;

pub struct Prompt {
    pub open: bool,
    pub input: String,
    pub selected: usize,
    pub completions: Vec<String>,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            open: false,
            input: String::new(),
            selected: 0,
            completions: vec![],
        }
    }

    pub fn example_content(&mut self) {
        self.open = true;
        self.selected = 2;
        let all = vec![
            "Open File",
            "Save File",
            "Close File",
            "Go to Line",
            "Toggle Theme",
            "Format Document",
        ];

        self.completions = all
            .into_iter()
            .filter(|s| s.to_lowercase().contains(&self.input.to_lowercase()))
            .map(|s| s.to_string())
            .collect();
    }

    pub fn update_completions(&mut self) {
        let all = vec![
            "Open File",
            "Save File",
            "Close File",
            "Go to Line",
            "Toggle Theme",
            "Format Document",
        ];

        self.completions = all
            .into_iter()
            .filter(|s| s.to_lowercase().contains(&self.input.to_lowercase()))
            .map(|s| s.to_string())
            .collect();

        self.selected = 0;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        // Ensure completions exist
        if self.completions.is_empty() {
            self.update_completions();
        }

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
                ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(35, 35, 38))
                        .rounding(egui::Rounding::same(6.0)) // less rounded
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70)))
                        .shadow(egui::epaint::Shadow::small_dark())
                        .show(ui, |ui| {
                            ui.set_width(width);

                            ui.add_space(14.0);

                            // 🔎 Search row
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);

                                ui.label(
                                    egui::RichText::new("🔎")
                                        .size(22.0)
                                        .color(egui::Color32::from_gray(180)),
                                );

                                ui.add_space(8.0);

                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut self.input)
                                        .hint_text("Type a command...")
                                        .frame(false)
                                        .font(egui::TextStyle::Heading)
                                        .desired_width(f32::INFINITY),
                                );

                                if response.changed() {
                                    self.update_completions();
                                }
                            });

                            ui.add_space(14.0);

                            if !self.completions.is_empty() {
                                ui.separator();

                                for (i, item) in self.completions.iter().enumerate() {
                                    let selected = i == self.selected;

                                    let bg = if selected {
                                        egui::Color32::from_rgb(70, 70, 75)
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };

                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), 38.0),
                                        egui::Sense::click(),
                                    );

                                    ui.painter().rect_filled(rect, 4.0, bg);

                                    ui.painter().text(
                                        rect.left_center() + egui::vec2(14.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        item,
                                        egui::TextStyle::Body.resolve(ui.style()),
                                        egui::Color32::WHITE,
                                    );

                                    if response.clicked() {
                                        self.selected = i;
                                    }

                                    // ui.add_space(4.0);
                                }
                            }
                        });
                });
            });
    }
}
