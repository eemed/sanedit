use std::collections::VecDeque;

use sanedit_messages::key::KeyEvent;
use sanedit_server::ClientId;

use crate::editor::Map;

#[derive(Debug, Clone)]
pub struct MacroReplay {
    pub keys: VecDeque<KeyEvent>,
    pub id: ClientId,
}

#[derive(Debug, Default)]
pub struct Macros {
    macros: Map<String, VecDeque<KeyEvent>>,
    pub replay: Option<MacroReplay>,
}

impl Macros {
    pub fn insert(&mut self, name: String, events: VecDeque<KeyEvent>) {
        self.macros.insert(name, events);
    }

    pub fn get(&self, name: &str) -> Option<&VecDeque<KeyEvent>> {
        self.macros.get(name)
    }

    pub fn names(&self) -> std::collections::hash_map::Keys<'_, String, VecDeque<KeyEvent>> {
        self.macros.keys()
    }

    pub fn is_replaying(&self) -> bool {
        self.replay.is_some()
    }
}
