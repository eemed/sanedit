mod context;

use std::cmp::min;

use anyhow::Result;
use sanedit_messages::{
    redraw::{Point, Size, Style},
    ClientMessage, Element, Message, MouseButton, MouseEvent,
};

use crate::{
    grid::{Grid, Rect, RedrawResult},
    terminal::Terminal,
};

pub(crate) use self::context::UIContext;

pub(crate) struct UI {
    terminal: Terminal,
    grid: Grid,
}

impl UI {
    pub fn new() -> Result<UI> {
        let terminal = Terminal::new()?;
        let width = terminal.width();
        let height = terminal.height();
        Ok(UI {
            terminal,
            grid: Grid::new(width, height),
        })
    }

    pub fn color_count(&self) -> usize {
        crossterm::style::available_color_count() as usize
    }

    pub fn window(&self) -> Rect {
        // Atleast size of 1 cell
        let mut rect = self.grid.window();
        rect.width = rect.width.max(1);
        rect.height = rect.height.max(1);
        rect
    }

    pub fn resize(&mut self, size: Size) -> anyhow::Result<()> {
        self.terminal.resize(size.width, size.height)?;
        self.grid.resize(size.width, size.height);

        // self.grid.draw(&self.context);
        // self.flush();
        Ok(())
    }

    /// Called when client will send input to server
    pub fn on_send_input(&mut self, msg: &Message) {
        self.grid.on_send_input(msg);
    }

    /// Called when client will send input to server
    pub fn on_focus_change(&mut self, focus: bool) {
        self.grid.on_focus_change(focus);
        let _ = self.flush();
    }

    pub fn handle_mouse_event(&mut self, mut ev: MouseEvent) -> Option<Message> {
        if let Some(ft) = self.grid.filetree() {
            if ft.rect.contains(&ev.point) {
                match ev.kind {
                    sanedit_messages::MouseEventKind::ScrollDown => {
                        ft.item.scroll = min(
                            ft.item.scroll + 2,
                            ft.item.items.items.len().saturating_sub(1),
                        );
                        let _ = self.flush();
                        return None;
                    }
                    sanedit_messages::MouseEventKind::ScrollUp => {
                        ft.item.scroll = ft.item.scroll.saturating_sub(2);
                        let _ = self.flush();
                        return None;
                    }
                    sanedit_messages::MouseEventKind::ButtonDown(MouseButton::Left) => {
                        ev.point = ev.point - ft.rect.position();
                        ev.point.y += ft.item.scroll;
                        ev.element = Element::Filetree;
                        return Some(Message::MouseEvent(ev));
                    }
                    _ => {}
                }
            }
        }

        if let Some(loc) = self.grid.locations() {
            if loc.rect.contains(&ev.point) {
                match ev.kind {
                    sanedit_messages::MouseEventKind::ScrollDown => {
                        loc.item.scroll = min(
                            loc.item.scroll + 2,
                            loc.item.items.items.len().saturating_sub(1),
                        );
                        let _ = self.flush();
                        return None;
                    }
                    sanedit_messages::MouseEventKind::ScrollUp => {
                        loc.item.scroll = loc.item.scroll.saturating_sub(2);
                        let _ = self.flush();
                        return None;
                    }
                    sanedit_messages::MouseEventKind::ButtonDown(MouseButton::Left) => {
                        ev.point = ev.point - loc.rect.position();
                        // -1 = header
                        ev.point.y += loc.item.scroll;
                        ev.point.y = ev.point.y.saturating_sub(1);
                        ev.element = Element::Locations;
                        return Some(Message::MouseEvent(ev));
                    }
                    _ => {}
                }
            }
        }

        let win = self.grid.window();
        if win.contains(&ev.point) {
            ev.point = ev.point - win.position();
            ev.element = Element::Window;
            return Some(Message::MouseEvent(ev));
        }

        None
    }

    pub fn handle_message(&mut self, msg: ClientMessage) -> anyhow::Result<UIResult> {
        use ClientMessage::*;
        match msg {
            Theme(theme) => {
                self.grid.theme = theme.into();
            }
            Redraw(msg) => match self.grid.handle_redraw(msg) {
                RedrawResult::Resized => return Ok(UIResult::Resize),
                RedrawResult::Ok => {}
            },
            Flush => {
                let _ = self.flush();
            }
            Bye => {
                log::info!("UI got bye, exiting.");
                return Ok(UIResult::Exit);
            }
            _ => {}
        }

        Ok(UIResult::Nothing)
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        // log::info!("Flush ui");
        let (cells, cursor) = self.grid.draw();
        for (line, row) in cells.iter().enumerate() {
            for (col, cell) in row.iter().enumerate() {
                self.terminal.draw_cell(cell, col, line)?;
            }
        }

        if let Some(cursor) = cursor {
            self.terminal.show_cursor()?;
            let Point { x, y } = cursor.point;
            self.terminal.set_style(Style {
                text_style: None,
                bg: cursor.bg,
                fg: cursor.fg,
            })?;
            self.terminal.set_cursor_style(cursor.shape)?;
            self.terminal.goto(x, y)?;
        } else {
            self.terminal.hide_cursor()?;
        }

        self.terminal.flush()?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum UIResult {
    Nothing,
    Exit,
    Resize,
}
