use eframe::egui;
use sanedit_messages::{
    ClientMessage, Message, Writer, redraw::{
        Popup, PopupComponent, Redraw, Size, Theme, ThemeField, prompt::{Prompt, PromptUpdate}, status::Status, window::WindowUpdate
    }
};

use crate::{
    input::keyevents_from_egui,
    ui::{grid::CharGrid, style::EguiStyle},
};

use std::{io::Write, sync::Arc};

use crossbeam::channel::{Receiver, Sender};

pub const TAB_HEIGHT: f32 = 32.0;

pub struct Tab<W: Write> {
    msg_recv: Receiver<Vec<ClientMessage>>,
    editor_writer: Writer<W, Message>,
    grid: CharGrid,
    pub status: Status,
    pub prompt: Option<Prompt>,
    pub popup: Option<Popup>,
    pub theme: Option<Arc<Theme>>,
    pub size: Option<Size>,
}

impl<W: Write> Tab<W> {
    pub fn new(msg_recv: Receiver<Vec<ClientMessage>>, editor_writer: Writer<W, Message>) -> Self {
        Self {
            msg_recv,
            editor_writer,
            grid: CharGrid::new(18.0),
            status: Status::default(),
            prompt: None,
            popup: None,
            theme: None,
            size: None,
        }
    }

    pub fn setup(&mut self, ctx: &egui::Context) {
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

    /// Draw the status bar at the top
    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Some(ref theme) = self.theme {
            let new_size = self.grid.size(ui);
            if self.size != Some(new_size) {
                self.size = Some(new_size);
                let _ = self.editor_writer.write(Message::Resize(new_size));
            }

            self.grid.show(ctx, ui, theme);
        }
    }

    pub fn redirect_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            let event = keyevents_from_egui(i);
            if let Some(event) = event {
                let _ = self.editor_writer.write(Message::KeyEvent(event));
            }
        });
    }

    pub fn process_messages(&mut self, ctx: &egui::Context) {
        while let Ok(messages) = self.msg_recv.try_recv() {
            for message in messages {
                self.handle_message(ctx, message);
            }
        }
    }

    fn handle_message(&mut self, ctx: &egui::Context, msg: ClientMessage) {
        match msg {
            ClientMessage::Hello { id } => {}
            ClientMessage::Theme(theme) => self.theme = Some(Arc::new(theme)),
            ClientMessage::Redraw(redraw) => self.handle_redraw(redraw),
            ClientMessage::SplitHorizontal => {}
            ClientMessage::SplitVertical => {}
            ClientMessage::ConnectionTest => {
                let _ = self.editor_writer.write(Message::ConnectionTest);
            }
            ClientMessage::Flush => {}
            ClientMessage::Bye => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn handle_redraw(&mut self, redraw: Redraw) {
        match redraw {
            Redraw::Window(window_update) => match window_update {
                WindowUpdate::Full(window) => self.grid.window = window,
                WindowUpdate::Cursor(cursor) => self.grid.window.cursor = cursor,
            },
            Redraw::Status(statusline) => self.status = statusline,
            Redraw::Prompt(prompt_update) => match prompt_update {
                PromptUpdate::Selection(sel) => {
                    if let Some(prompt) = &mut self.prompt {
                        prompt.selected = sel;
                    }
                }
                PromptUpdate::Full(prompt) => self.prompt = Some(prompt),
                PromptUpdate::Close => self.prompt = None,
            },
            Redraw::Completion(completion_update) => {}
            Redraw::Filetree(items_update) => {}
            Redraw::Locations(items_update) => {}
            Redraw::Snapshots(snapshots_update) => {}
            Redraw::StatusMessage(status_message) => {}
            Redraw::Popup(popup_component) => match popup_component {
                PopupComponent::Open(popup) => self.popup = Some(popup),
                PopupComponent::Close => self.popup = None,
            },
        }
    }
}
