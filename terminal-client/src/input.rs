use std::{sync::mpsc, time::Duration};

use anyhow::Result;
use crossterm::event::{poll, read, KeyCode, KeyModifiers, MouseEventKind};
use sanedit_messages::{redraw::Size, Key, KeyEvent, KeyMods, Message, MouseEvent};

use crate::message::ClientInternalMessage;

const POLL_DURATION: Duration = Duration::from_millis(100);
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
        if poll(POLL_DURATION)? {
            let event = read()?;
            process_input_event(event, sender)?;
        }
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
            let msg: Option<Message> = match mouse_event.kind {
                MouseEventKind::ScrollDown => Some(MouseEvent::ScrollDown.into()),
                MouseEventKind::ScrollUp => Some(MouseEvent::ScrollDown.into()),
                _ => None,
            };

            if let Some(msg) = msg {
                sender.send(msg.into())?;
            }
        }
    }

    Ok(())
}

pub(crate) fn convert_key_event(key: crossterm::event::KeyEvent) -> sanedit_messages::KeyEvent {
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

    let mut mods = KeyMods::empty();

    if key.modifiers.contains(KeyModifiers::ALT) {
        mods |= KeyMods::ALT;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        mods |= KeyMods::CONTROL;
    }

    KeyEvent::new(plain_key, mods)
}
