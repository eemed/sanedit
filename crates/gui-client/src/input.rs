use eframe::egui;
use egui::{Context, Event, Key as EKey};
use sanedit_messages::key::{self, Key, KeyEvent};

pub fn keyevents_from_egui(i: &egui::InputState) -> Vec<KeyEvent> {
    i.events
        .iter()
        .filter_map(|event| match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let mut key = egui_key_to_key(*key)?;

                let mut mods = 0;

                if modifiers.ctrl {
                    mods |= key::CONTROL;
                }
                if modifiers.alt {
                    mods |= key::ALT;
                }
                if modifiers.shift {
                    mods |= key::SHIFT;
                }

                if modifiers.shift && key == Key::Tab {
                    key = Key::BackTab;
                }

                Some(KeyEvent::new(key, mods))
            }
            // Event::Text(text) => text
            //     .chars()
            //     .next()
            //     .map(|ch| KeyEvent::new(Key::Char(ch), 0)),
            _ => None,
        })
        .collect()
}

fn egui_key_to_key(key: egui::Key) -> Option<Key> {
    use egui::Key::*;

    Some(match key {
        A => Key::Char('a'),
        B => Key::Char('b'),
        C => Key::Char('c'),
        D => Key::Char('d'),
        E => Key::Char('e'),
        F => Key::Char('f'),
        G => Key::Char('g'),
        H => Key::Char('h'),
        I => Key::Char('i'),
        J => Key::Char('j'),
        K => Key::Char('k'),
        L => Key::Char('l'),
        M => Key::Char('m'),
        N => Key::Char('n'),
        O => Key::Char('o'),
        P => Key::Char('p'),
        Q => Key::Char('q'),
        R => Key::Char('r'),
        S => Key::Char('s'),
        T => Key::Char('t'),
        U => Key::Char('u'),
        V => Key::Char('v'),
        W => Key::Char('w'),
        X => Key::Char('x'),
        Y => Key::Char('y'),
        Z => Key::Char('z'),
        Enter => Key::Enter,
        Escape => Key::Esc,
        Tab => Key::Tab,
        Backspace => Key::Backspace,
        Delete => Key::Delete,
        ArrowUp => Key::Up,
        ArrowDown => Key::Down,
        ArrowLeft => Key::Left,
        ArrowRight => Key::Right,
        Home => Key::Home,
        End => Key::End,
        PageUp => Key::PageUp,
        PageDown => Key::PageDown,
        Insert => Key::Insert,
        F1 => Key::F(1),
        F2 => Key::F(2),
        F3 => Key::F(3),
        F4 => Key::F(4),
        F5 => Key::F(5),
        F6 => Key::F(6),
        F7 => Key::F(7),
        F8 => Key::F(8),
        F9 => Key::F(9),
        F10 => Key::F(10),
        F11 => Key::F(11),
        F12 => Key::F(12),
        _ => return None,
        // Space => todo!(),
        // Minus => todo!(),
        // PlusEquals => todo!(),
        // Num0 => todo!(),
        // Num1 => todo!(),
        // Num2 => todo!(),
        // Num3 => todo!(),
        // Num4 => todo!(),
        // Num5 => todo!(),
        // Num6 => todo!(),
        // Num7 => todo!(),
        // Num8 => todo!(),
        // Num9 => todo!(),
        // F13 => todo!(),
        // F14 => todo!(),
        // F15 => todo!(),
        // F16 => todo!(),
        // F17 => todo!(),
        // F18 => todo!(),
        // F19 => todo!(),
        // F20 => todo!(),
    })
}
