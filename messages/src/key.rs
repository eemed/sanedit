use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct KeyEvent {
    pub(crate) key: Key,
    pub(crate) mods: KeyMods,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct KeyMods: u8 {
        const CONTROL = 0b00_00_10;
        const ALT = 0b00_01_00;
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
