use std::{sync::mpsc, time::Duration};

use anyhow::Result;
use crossterm::event::{poll, read, KeyCode, KeyModifiers};
use sanedit_messages::{
    key::{self, Key, KeyEvent, KeyMods},
    redraw::{Point, Size},
    Message, MouseButton, MouseEvent, MouseEventKind,
};

use crate::message::ClientInternalMessage;

const RE_RESIZE_POLL_DURATION: Duration = Duration::from_millis(100);

pub(crate) fn run_loop(mut sender: mpsc::Sender<ClientInternalMessage>) {
    let msg = match run_loop_impl(&mut sender) {
        Ok(_) => ClientInternalMessage::Bye,
        Err(e) => ClientInternalMessage::Error(e.to_string()),
    };

    let _ = sender.send(msg);
}

pub(crate) fn run_loop_impl(sender: &mut mpsc::Sender<ClientInternalMessage>) -> Result<()> {
    loop {
        let event = read()?;
        process_input_event(event, sender)?;
    }
}

fn process_input_event(
    event: crossterm::event::Event,
    sender: &mut mpsc::Sender<ClientInternalMessage>,
) -> Result<()> {
    use crossterm::event::Event::*;

    match event {
        Key(key_event) => {
            let key = convert_key_event(key_event);
            sender.send(Message::KeyEvent(key).into())?;
        }
        Resize(mut width, mut height) => {
            while poll(RE_RESIZE_POLL_DURATION)? {
                let e = read()?;
                match e {
                    Resize(w, h) => {
                        width = w;
                        height = h;
                    }
                    _ => {
                        sender.send(
                            Message::Resize(Size {
                                width: width as usize,
                                height: height as usize,
                            })
                            .into(),
                        )?;
                        process_input_event(e, sender)?;
                        return Ok(());
                    }
                }
            }

            sender.send(
                Message::Resize(Size {
                    width: width as usize,
                    height: height as usize,
                })
                .into(),
            )?;
        }
        Mouse(mouse_event) => {
            if let Some(msg) = convert_mouse_event(mouse_event) {
                sender.send(Message::MouseEvent(msg).into())?;
            }
        }
    }

    Ok(())
}

pub(crate) fn convert_key_event(
    key: crossterm::event::KeyEvent,
) -> sanedit_messages::key::KeyEvent {
    let plain_key = match key.code {
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::F(n) => Key::F(n),
        KeyCode::Char(ch) => Key::Char(ch),
        KeyCode::Esc => Key::Esc,
        KeyCode::Null => Key::Unknown,
    };

    let mods = convert_mods(&key.modifiers);

    KeyEvent::new(plain_key, mods)
}

fn convert_mods(modifiers: &KeyModifiers) -> KeyMods {
    let mut mods = 0u8;

    if modifiers.contains(KeyModifiers::ALT) {
        mods |= key::ALT;
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        mods |= key::CONTROL;
    }

    if modifiers.contains(KeyModifiers::SHIFT) {
        mods |= key::SHIFT;
    }

    mods
}

pub(crate) fn convert_mouse_event(
    event: crossterm::event::MouseEvent,
) -> Option<sanedit_messages::MouseEvent> {
    let kind = match event.kind {
        crossterm::event::MouseEventKind::Down(b) => {
            MouseEventKind::ButtonDown(convert_mouse_button(b))
        }
        crossterm::event::MouseEventKind::Up(b) => {
            MouseEventKind::ButtonUp(convert_mouse_button(b))
        }
        crossterm::event::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        crossterm::event::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        crossterm::event::MouseEventKind::Drag(_b) => return None,
        crossterm::event::MouseEventKind::Moved => return None,
    };

    let mods = convert_mods(&event.modifiers);
    let point = Point {
        x: event.column as usize,
        y: event.row as usize,
    };

    Some(MouseEvent { kind, mods, point })
}

fn convert_mouse_button(btn: crossterm::event::MouseButton) -> MouseButton {
    match btn {
        crossterm::event::MouseButton::Left => MouseButton::Left,
        crossterm::event::MouseButton::Right => MouseButton::Right,
        crossterm::event::MouseButton::Middle => MouseButton::Middle,
    }
}
