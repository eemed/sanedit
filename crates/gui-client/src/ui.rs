mod cell;
mod filetree;
mod floating;
mod grid;
mod select;
mod settings;
mod statusbar;
mod style;
mod tab;

use std::{io::Write, sync::Arc};

use crossbeam::channel::{Receiver, Sender};
use eframe::egui::{self, Button};
use sanedit_messages::{
    redraw::{Theme, ThemeField},
    ClientMessage, Message, Writer,
};

use crate::ui::{
    filetree::Filetree,
    floating::Floating,
    select::Select,
    settings::Settings,
    statusbar::StatusBar,
    style::EguiStyle,
    tab::{Tab, TAB_HEIGHT},
};

struct UI<W: Write> {
    sender: Option<Sender<egui::Context>>,
    active_tab: usize,
    tabs: Vec<Tab<W>>,
    status: StatusBar,
    select: Select,
    floating: Floating,
    filetree: Filetree,
    settings: Settings,
}

impl<W: Write> UI<W> {
    fn name() -> &'static str {
        "SanEdit"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(ctx.clone());

            for tab in &mut self.tabs {
                tab.setup(ctx, &self.settings);
            }
        }
    }

    fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        let tab = &self.tabs[self.active_tab];
        let theme = tab.theme.clone();
        let style = theme
            .as_ref()
            .map(|theme| EguiStyle::from(theme.get(ThemeField::Default)))
            .unwrap_or(EguiStyle::default());
        let comment_style = theme
            .as_ref()
            .map(|theme| EguiStyle::from(theme.get(ThemeField::Comment)))
            .unwrap_or(EguiStyle::default());

        // Borders
        ctx.style_mut(|estyle| {
            estyle.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, comment_style.fg);
        });

        if let Some(ref theme) = theme {
            self.status.show(ctx, &tab.status, &self.settings, theme);

            let tab = &self.tabs[self.active_tab];
            if let Some(ref ft) = tab.filetree_items {
                self.filetree.show(ctx, ft, &self.settings, theme);
            }

            self.tab_bar(ctx, theme);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: style.bg,
                inner_margin: egui::Margin::same(2),
                ..Default::default()
            })
            .show(ctx, |ui| {
                let tab = &mut self.tabs[self.active_tab];
                tab.show(ctx, ui, &self.settings);

                let tab = &mut self.tabs[self.active_tab];
                if let Some(cell_size) = tab.cell_size() {
                    if let Some(ref theme) = theme {
                        if let Some(ref prompt) = tab.prompt {
                            self.select.show(ctx, prompt, &self.settings, theme);
                        }
                        if let Some(ref popup) = tab.popup {
                            self.floating.show(ctx, ui, popup, &self.settings, theme, cell_size);
                        }

                        self.settings.show(ctx, theme);
                    }
                }
            });
    }

    fn tab_bar(&mut self, ctx: &egui::Context, theme: &Theme) {
        fn tab_button(
            ui: &mut egui::Ui,
            selected: bool,
            label: &str,
            style: &EguiStyle,
            font_id: egui::FontId,
        ) -> egui::Response {
            let label = egui::RichText::new(label).color(style.fg).font(font_id);
            let button = Button::new(label)
                .min_size(egui::vec2(80.0, TAB_HEIGHT))
                .fill(style.bg)
                .stroke(egui::Stroke::NONE)
                .frame(false)
                .selected(selected);

            ui.add(button)
        }

        let inactive_style = EguiStyle::from(theme.get(ThemeField::Statusline));
        let sel_style = EguiStyle::from(theme.get(ThemeField::Default));
        egui::TopBottomPanel::top("tab_bar")
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame {
                fill: inactive_style.bg,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let font_id = self.settings.ui_font_id(ui);
                    for tab in &self.tabs {
                        let selected = self.active_tab == 0;
                        let style = if selected {
                            &sel_style
                        } else {
                            &inactive_style
                        };
                        let splits: Vec<&str> = tab.status.buffer.split("/").collect();

                        egui::Frame::default()
                            .inner_margin(egui::Margin::symmetric(8, 0))
                            .fill(style.bg)
                            .show(ui, |ui| {
                                if tab_button(
                                    ui,
                                    self.active_tab == 0,
                                    &splits[splits.len() - 1],
                                    style,
                                    font_id.clone(),
                                )
                                .clicked()
                                {
                                    self.active_tab = 0;
                                }
                            });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button = ui.add_sized(
                            (ui.available_height(), ui.available_height()),
                            egui::Button::new(
                                egui::RichText::new("🔧")
                                    .font(font_id)
                                    .color(inactive_style.fg),
                            )
                            .frame(false)
                            .fill(inactive_style.bg)
                            .corner_radius(0),
                        );

                        // if button.hovered() {
                        //     let painter = ui.painter_at(button.rect);
                        //     painter.rect_filled(button.rect, 4.0, sel_style.bg);
                        // }

                        if button.clicked() {
                            self.settings.open = true;
                        }
                    });
                });
            });
    }

    /// Returns whether the input was consumed
    fn process_clientside_input(&mut self, ctx: &egui::Context) -> bool {
        false
    }
}

impl<W: Write> eframe::App for UI<W> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.setup(ctx);
        if !self.process_clientside_input(ctx) {
            let tab = &mut self.tabs[self.active_tab];
            tab.process_messages(ctx);
            tab.redirect_input(ctx);
        }

        self.draw(ctx, frame);
    }
}

pub(crate) fn run<W: Write + 'static>(
    ctx_send: Sender<egui::Context>,
    msg_recv: Receiver<Vec<ClientMessage>>,
    writer: Writer<W, Message>,
) {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((1200.0, 1000.0)),
        renderer: eframe::Renderer::Glow,
        vsync: false,
        ..eframe::NativeOptions::default()
    };

    let settings = Settings::new();
    let status = StatusBar::new(30.0);
    let select = Select::new();
    let floating = Floating::new();
    let filetree = Filetree::new(600.0);
    let tab = Tab::new(msg_recv, writer);

    let _ = eframe::run_native(
        UI::<W>::name(),
        native_options,
        Box::new(move |_| {
            Ok(Box::new(UI {
                sender: Some(ctx_send),
                tabs: vec![tab],
                active_tab: 0,
                status,
                select,
                floating,
                filetree,
                settings,
            }))
        }),
    );
}
