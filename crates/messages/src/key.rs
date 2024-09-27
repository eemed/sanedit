use std::fmt;

use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

pub type KeyMods = u8;
pub const CONTROL: u8 = 1 << 0;
pub const ALT: u8 = 1 << 1;
pub const SHIFT: u8 = 1 << 2;

/// Keyboard keys
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub enum Key {
    Char(char),
    F(u8),
    Enter,
    Esc,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct KeyEvent {
    key: Key,
    mods: KeyMods,
}

impl KeyEvent {
    pub fn new(key: Key, mods: KeyMods) -> KeyEvent {
        KeyEvent { key, mods }
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn control_pressed(&self) -> bool {
        self.mods & CONTROL != 0
    }

    pub fn alt_pressed(&self) -> bool {
        self.mods & ALT != 0
    }
}

/// Separator for different key presses.
pub(crate) const KEY_PRESS_SEPARATOR: &str = " ";

/// Separator for different keys in a single key press.
pub(crate) const KEY_SEPARATOR: &str = "+";

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctrl = if self.mods & CONTROL != 0 {
            format!("ctrl{}", KEY_SEPARATOR)
        } else {
            "".to_string()
        };

        let alt = if self.mods & ALT != 0 {
            format!("alt{}", KEY_SEPARATOR)
        } else {
            "".to_string()
        };

        let shift = if self.mods & SHIFT != 0 {
            format!("shift{}", KEY_SEPARATOR)
        } else {
            "".to_string()
        };

        let key = match self.key {
            Key::Char(ch) => ch.to_string(),
            Key::F(n) => format!("F{}", n),
            Key::Enter => "enter".to_string(),
            Key::Esc => "esc".to_string(),
            Key::Tab => "tab".to_string(),
            Key::BackTab => "btab".to_string(),
            Key::Up => "up".to_string(),
            Key::Down => "down".to_string(),
            Key::Left => "left".to_string(),
            Key::Right => "right".to_string(),
            Key::Backspace => "backspace".to_string(),
            Key::Delete => "delete".to_string(),
            Key::Home => "home".to_string(),
            Key::End => "end".to_string(),
            Key::PageUp => "pageup".to_string(),
            Key::PageDown => "pagedown".to_string(),
            Key::Insert => "insert".to_string(),
            // Key::Unknown => {}
            _ => "???".to_string(),
        };

        f.write_fmt(format_args!("{}{}{}{}", ctrl, alt, shift, key))
    }
}

impl TryFrom<&str> for KeyEvent {
    type Error = String;

    fn try_from(string: &str) -> Result<KeyEvent, String> {
        let keys = string.split(KEY_SEPARATOR);
        let mut mods = 0u8;
        let mut seen = FxHashSet::default();

        for token in keys {
            if seen.contains(&token) {
                return Err(format!(
                    "Keybinding contains multiple same tokens {}",
                    string
                ));
            }

            seen.insert(token);

            match token {
                "alt" => {
                    mods |= ALT;
                    continue;
                }
                "ctrl" | "ctl" => {
                    mods |= CONTROL;
                    continue;
                }
                "shift" => {
                    mods |= SHIFT;
                    continue;
                }
                token => {
                    let key = if token.chars().count() == 1 {
                        let ch = token.chars().next().unwrap();
                        if ch.is_uppercase() {
                            mods |= SHIFT;
                        }
                        Key::Char(ch)
                    } else if token.starts_with('f') || token.starts_with('F') {
                        let mut chars = token.chars();
                        // skip f
                        chars.next();

                        let string = chars.fold(String::new(), |mut acc, c| {
                            acc.push(c);
                            acc
                        });

                        match u8::from_str_radix(&string, 10) {
                            Ok(fkey) => Key::F(fkey),
                            Err(_) => {
                                return Err(format!(
                                    "Failed to parse function keybinding number {}",
                                    token
                                ))
                            }
                        }
                    } else {
                        match token {
                            "enter" => Key::Enter,
                            "esc" => Key::Esc,
                            "tab" => Key::Tab,
                            "btab" | "backtab" => {
                                mods |= SHIFT;
                                Key::BackTab
                            }
                            "insert" => Key::Insert,
                            "delete" => Key::Delete,
                            "home" => Key::Home,
                            "end" => Key::End,
                            "pageup" | "pgup" => Key::PageUp,
                            "pagedown" | "pgdown" => Key::PageDown,
                            "up" => Key::Up,
                            "down" => Key::Down,
                            "left" => Key::Left,
                            "right" => Key::Right,
                            "space" => Key::Char(' '),
                            "backspace" | "bs" => Key::Backspace,
                            _ => return Err(format!("Failed to parse keybinding {}", string)),
                        }
                    };

                    return Ok(KeyEvent::new(key, mods));
                }
            };
        }

        Err(format!("Failed to parse keybinding {}", string))
    }
}

pub fn keyevents_to_string(events: &[KeyEvent]) -> String {
    let mut result = String::new();
    for event in events {
        if !result.is_empty() {
            result.push_str(KEY_PRESS_SEPARATOR);
        }

        let es = format!("{}", event);
        result.push_str(&es);
    }
    result
}

pub fn try_parse_keyevents(string: &str) -> Result<Vec<KeyEvent>, String> {
    let key_events = string.split(KEY_PRESS_SEPARATOR);

    let mut events = vec![];
    for event in key_events {
        events.push(KeyEvent::try_from(event)?);
    }
    Ok(events)
}
