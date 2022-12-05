use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct KeyMods: u8 {
        const CONTROL = 0b00_00_01;
        const ALT = 0b00_00_10;
    }
}

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
    pub(crate) key: Key,
    pub(crate) mods: KeyMods,
}

impl KeyEvent {
    pub fn new(key: Key, mods: KeyMods) -> KeyEvent {
        KeyEvent { key, mods }
    }
}
