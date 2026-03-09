use eframe::egui;
use egui::Event;
use sanedit_messages::key::{self, Key, KeyEvent};

pub fn keyevents_from_egui(i: &mut egui::InputState) -> Option<KeyEvent> {
    // let event = i.events.first()?;
    //         println!("Event: {:?}", i.events);
    for event in &i.events {
        match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if let Some(mut key) = egui_key_to_key(*key) {
                    let mut mods = 0;

                    if modifiers.ctrl {
                        mods |= key::CONTROL;
                    }
                    if modifiers.alt {
                        mods |= key::ALT;
                    }
                    if modifiers.shift {
                        mods |= key::SHIFT;

                        if let Key::Char(ch) = &mut key {
                            ch.make_ascii_uppercase();
                        }
                    }

                    if modifiers.shift && key == Key::Tab {
                        key = Key::BackTab;
                    }

                    // println!("Key: {key}, mods: {mods}");

                    i.events.clear();
                    return Some(KeyEvent::new(key, mods));
                }
            }
            Event::Text(text) => {
                let event = text.chars().next().map(|ch| {
                    let mut mods = 0;
                    if ch.is_uppercase() {
                        mods |= key::SHIFT;
                    }

                    let key = Key::Char(ch);
                    // println!("Key2: {key}, mods: {mods}");

                    KeyEvent::new(key, mods)
                });
                if event.is_some() {
                    i.events.clear();
                    return event;
                }
            }
            _ => {}
        }
    }

    None
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
        Space => Key::Char(' '),
        Num0 => Key::Char('0'),
        Num1 => Key::Char('1'),
        Num2 => Key::Char('2'),
        Num3 => Key::Char('3'),
        Num4 => Key::Char('4'),
        Num5 => Key::Char('5'),
        Num6 => Key::Char('6'),
        Num7 => Key::Char('7'),
        Num8 => Key::Char('8'),
        Num9 => Key::Char('9'),
        // Minus => Key::Char('-'),
        _ => return None,
    })
}
