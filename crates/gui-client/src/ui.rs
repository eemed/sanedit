mod cell;
mod grid;
mod select;
mod statusbar;
mod style;

use std::io::Write;

use crossbeam::channel::{Receiver, Sender};
use eframe::egui;
use sanedit_messages::{
    redraw::{prompt::PromptUpdate, window::WindowUpdate, Redraw, Theme},
    ClientMessage, Message, Writer,
};

use crate::{
    input::keyevents_from_egui,
    ui::{grid::CharGrid, select::Select, statusbar::StatusBar},
};

struct UI<W: Write> {
    sender: Option<Sender<egui::Context>>,
    msg_recv: Receiver<Vec<ClientMessage>>,
    editor_writer: Writer<W, Message>,

    grid: CharGrid,
    status: StatusBar,
    select: Select,
    theme: Option<Theme>,
}

impl<W: Write> UI<W> {
    fn name() -> &'static str {
        "SanEdit"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        if let Some(sender) = self.sender.take() {
            setup_fonts(ctx);
            let _ = sender.send(ctx.clone());

            egui::CentralPanel::default().show(ctx, |ui| {
                let size = self.grid.size(ui);
                self.editor_writer
                    .write(Message::Hello {
                        color_count: 16_777_216,
                        size,
                        parent: None,
                    })
                    .expect("Failed to send hello");
            });
        }
    }

    fn draw(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Start from dark theme
        ctx.set_visuals(egui::Visuals::dark());

        // Force window background to black
        let mut visuals = ctx.style().visuals.clone();
        visuals.panel_fill = egui::Color32::BLACK;
        visuals.extreme_bg_color = egui::Color32::BLACK;
        ctx.set_visuals(visuals);

        if let Some(ref theme) = self.theme {
            self.status.show(ctx, theme);

            egui::CentralPanel::default().show(ctx, |ui| {
                self.grid.show(ui, theme);
            });

            self.select.show(ctx, theme);
        }
    }

    fn redirect_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            let events = keyevents_from_egui(i);
            for event in events {
                let _ = self.editor_writer.write(Message::KeyEvent(event));
            }
        });
    }

    fn process_messages(&mut self) {
        while let Ok(messages) = self.msg_recv.try_recv() {
            for message in messages {
                self.handle_message(message);
            }
        }
    }

    fn handle_message(&mut self, msg: ClientMessage) {
        match msg {
            ClientMessage::Hello { id } => {}
            ClientMessage::Theme(theme) => self.theme = Some(theme),
            ClientMessage::Redraw(redraw) => self.handle_redraw(redraw),
            ClientMessage::SplitHorizontal => {}
            ClientMessage::SplitVertical => {}
            ClientMessage::ConnectionTest => {
                let _ = self.editor_writer.write(Message::ConnectionTest);
            }
            ClientMessage::Flush => {}
            ClientMessage::Bye => {
            }
        }
    }

    fn handle_redraw(&mut self, redraw: Redraw) {
        match redraw {
            Redraw::Window(window_update) => match window_update {
                WindowUpdate::Full(window) => self.grid.window = window,
                WindowUpdate::Cursor(cursor) => self.grid.window.cursor = cursor,
            },
            Redraw::Statusline(statusline) => self.status.statusline = statusline,
            Redraw::Prompt(prompt_update) => match prompt_update {
                PromptUpdate::Selection(sel) => {
                    if let Some(prompt) = &mut self.select.prompt {
                        prompt.selected = sel;
                    }
                }
                PromptUpdate::Full(prompt) => self.select.prompt = Some(prompt),
                PromptUpdate::Close => self.select.prompt = None,
            },
            Redraw::Completion(completion_update) => {}
            Redraw::Filetree(items_update) => {}
            Redraw::Locations(items_update) => {}
            Redraw::Snapshots(snapshots_update) => {}
            Redraw::StatusMessage(status_message) => {}
            Redraw::Popup(popup_component) => {}
        }
    }
}

impl<W: Write> eframe::App for UI<W> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.setup(ctx);
        self.process_messages();
        self.redirect_input(ctx);
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

    let grid = CharGrid::new(18.0);
    let status = StatusBar::new();
    let prompt = Select::new(16.0);

    let _ = eframe::run_native(
        UI::<W>::name(),
        native_options,
        Box::new(|_| {
            Box::new(UI {
                sender: Some(ctx_send),
                msg_recv,
                editor_writer: writer,
                grid,
                status,
                select: prompt,
                theme: None,
            })
        }),
    );
}
