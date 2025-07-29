use std::sync::mpsc::Sender;

use sanedit_messages::{key::KeyEvent, redraw::{window::Window, Theme}};

pub(crate) mod snake;

pub(crate) trait Game: std::fmt::Debug {
    fn handle_input(&mut self, key_event: KeyEvent) -> bool;
    fn tick(&mut self);
    fn set_tick_sender(&mut self, tick_sender: Sender<u64>);
    fn draw(&self, cells: &mut Window, theme: &Theme);
}
