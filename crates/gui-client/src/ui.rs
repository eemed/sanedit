mod cell;
mod floating;
mod grid;
mod select;
mod statusbar;
mod style;
mod tab;

use std::io::Write;

use crossbeam::channel::{Receiver, Sender};
use eframe::egui::{self, Button};
use sanedit_messages::{
    redraw::{
        prompt::PromptUpdate, window::WindowUpdate, PopupComponent, Redraw, Theme, ThemeField,
    },
    ClientMessage, Message, Writer,
};

use crate::{
    input::keyevents_from_egui,
    ui::{
        floating::Floating,
        grid::CharGrid,
        select::Select,
        statusbar::StatusBar,
        style::EguiStyle,
        tab::{Tab, TAB_HEIGHT},
    },
};

struct UI<W: Write> {
    sender: Option<Sender<egui::Context>>,
    active_tab: usize,
    tabs: Vec<Tab<W>>,
    status: StatusBar,
    select: Select,
    floating: Floating,
}

impl<W: Write> UI<W> {
    fn name() -> &'static str {
        "SanEdit"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        if let Some(sender) = self.sender.take() {
            setup_fonts(ctx);
            let _ = sender.send(ctx.clone());

            for tab in &mut self.tabs {
                tab.setup(ctx);
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

        if let Some(ref theme) = theme {
            self.status.show(ctx, &tab.status, theme);
            self.tab_bar(ctx, theme);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: style.bg,
                inner_margin: egui::Margin::same(2.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                let tab = &mut self.tabs[self.active_tab];
                tab.show(ctx, ui);
            });

        let tab = &mut self.tabs[self.active_tab];
        if let Some(ref theme) = theme {
            if let Some(ref prompt) = tab.prompt {
                self.select.show(ctx, prompt, theme);
            }
            if let Some(ref popup) = tab.popup {
                self.floating.show(ctx, popup, theme);
            }
        }
    }

    fn tab_bar(&mut self, ctx: &egui::Context, theme: &Theme) {
        fn tab_button(
            ui: &mut egui::Ui,
            selected: bool,
            label: &str,
            style: &EguiStyle,
        ) -> egui::Response {
            let label = egui::RichText::new(label).color(style.fg);
            let button = Button::new(label)
                .min_size(egui::vec2(120.0, TAB_HEIGHT))
                .frame(true)
                .fill(style.bg)
                .selected(selected);

            ui.add(button)
        }

        fn add_tab_button(
            ui: &mut egui::Ui,
            selected: bool,
            label: &str,
            style: &EguiStyle,
        ) -> egui::Response {
            let label = egui::RichText::new(label).color(style.fg);
            let button = Button::new(label)
                .min_size(egui::vec2(TAB_HEIGHT, TAB_HEIGHT))
                .frame(true)
                .fill(style.bg)
                .selected(selected);

            ui.add(button)
        }

        let style = EguiStyle::from(theme.get(ThemeField::Statusline));
        egui::TopBottomPanel::top("tab_bar")
            .resizable(false)
            .frame(egui::Frame {
                fill: style.bg,
                inner_margin: egui::Margin::same(4.0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for tab in &self.tabs {
                        if tab_button(ui, self.active_tab == 0, &tab.status.buffer, &style)
                            .clicked()
                        {
                            self.active_tab = 0;
                        }
                    }

                    if add_tab_button(ui, self.active_tab == 2, "+", &style).clicked() {}
                });
            });
    }
}

impl<W: Write> eframe::App for UI<W> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.setup(ctx);
        let tab = &mut self.tabs[self.active_tab];
        tab.process_messages(ctx);
        tab.redirect_input(ctx);

        self.draw(ctx, frame);
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Load JetBrains Mono
    fonts.font_data.insert(
        "comicmono".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/ComicMono/ComicMono.ttf")),
    );

    // Put it first in monospace family
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "comicmono".to_owned());

    ctx.set_fonts(fonts);
}

pub(crate) fn run<W: Write + 'static>(
    ctx_send: Sender<egui::Context>,
    msg_recv: Receiver<Vec<ClientMessage>>,
    writer: Writer<W, Message>,
) {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((1200.0, 1000.0)),
        ..eframe::NativeOptions::default()
    };

    let status = StatusBar::new();
    let select = Select::new(16.0);
    let floating = Floating::new(16.0);
    let tab = Tab::new(msg_recv, writer);

    let _ = eframe::run_native(
        UI::<W>::name(),
        native_options,
        Box::new(|_| {
            Box::new(UI {
                sender: Some(ctx_send),
                tabs: vec![tab],
                active_tab: 0,
                status,
                select,
                floating,
            })
        }),
    );
}
