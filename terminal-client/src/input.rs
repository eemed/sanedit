use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use crossterm::event::{poll, read, KeyCode, KeyModifiers, MouseEventKind};
use sanedit_messages::{Key, KeyEvent, KeyMods, Message, MouseEvent, Writer};

const POLL_DURATION: Duration = Duration::from_millis(100);
const RE_RESIZE_POLL_DURATION: Duration = Duration::from_millis(100);

pub(crate) fn run_loop<W: io::Write>(write: W, stop: Arc<AtomicBool>) -> Result<()> {
    let mut writer: Writer<_, Message> = Writer::new(write);

    loop {
        if poll(POLL_DURATION)? {
            let event = read()?;
            if let Err(e) = process_input_event(event, &mut writer) {
                log::error!("Client failed to send event {:?} to server: {}", event, e);
            }
        }

        if stop.load(Ordering::Acquire) {
            break;
        }
    }

    Ok(())
}

fn process_input_event<W: io::Write>(
    event: crossterm::event::Event,
    writer: &mut Writer<W, Message>,
) -> Result<()> {
    use crossterm::event::Event::*;

    match event {
        Key(key_event) => {
            let key = convert_key_event(key_event);
            writer.write(Message::KeyEvent(key))?;
        }
        Resize(mut width, mut height) => {
            // while poll(RE_RESIZE_POLL_DURATION)? {
            //     let e = read()?;
            //     match e {
            //         Resize(w, h) => {
            //             width = w;
            //             height = h;
            //         }
            //         _ => {
            //             resize(width as usize, height as usize, handle)?;
            //             process_input_event(e, handle)?;
            //             return Ok(());
            //         }
            //     }
            // }

            // resize(width as usize, height as usize, handle)?;
        }
        Mouse(mouse_event) => {
            let msg: Option<Message> = match mouse_event.kind {
                MouseEventKind::ScrollDown => Some(MouseEvent::ScrollDown.into()),
                MouseEventKind::ScrollUp => Some(MouseEvent::ScrollDown.into()),
                _ => None,
            };

            if let Some(msg) = msg {
                writer.write(msg)?;
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
