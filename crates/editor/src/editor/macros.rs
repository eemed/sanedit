use sanedit_messages::key::KeyEvent;

use crate::editor::Map;

#[derive(Debug, Default)]
pub struct Macros {
    macros: Map<String, Vec<KeyEvent>>,
}

impl Macros {
    pub fn push(&mut self, name: String, events: Vec<KeyEvent>) {
        self.macros.insert(name, events);
    }

    pub fn get(&self, name: &str) -> Option<&[KeyEvent]> {
        self.macros.get(name).map(Vec::as_slice)
    }

    pub fn names(&self) -> std::collections::hash_map::Keys<'_, String, Vec<KeyEvent>> {
        self.macros.keys()
    }
}
