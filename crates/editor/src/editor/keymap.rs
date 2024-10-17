use rustc_hash::FxHashMap;
use sanedit_messages::key::KeyEvent;
use strum_macros::AsRefStr;

use crate::actions::Action;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum KeymapKind {
    Search,
    Prompt,
    Window,
    Completion,
    Filetree,
    Locations,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Keymap {
    root: KeyTrie,
}

impl Keymap {
    /// Get a binding result for events.
    /// The result may be
    /// Matched => found a binding for events and its action
    /// NotFound => no binding for key combination
    /// Pending => need more input to decide
    pub fn get(&self, events: &[KeyEvent]) -> KeymapResult {
        self.root.get(events)
    }

    /// Create a new binding for key combination events.
    pub fn bind(&mut self, events: &[KeyEvent], action: Action) {
        self.root.bind(events, action);
    }

    /// Find the keybinding mapped to action with name
    pub fn find_bound_key(&self, name: &str) -> Option<Vec<KeyEvent>> {
        let root = &self.root.root;
        root.find_bound_key(name)
    }
}

#[derive(Debug)]
pub(crate) enum KeymapResult {
    Matched(Action),
    Pending,
    NotFound,
}

#[derive(Debug, Clone)]
struct KeyTrie {
    root: KeyTrieNode,
}

impl KeyTrie {
    fn get(&self, events: &[KeyEvent]) -> KeymapResult {
        self.root.get(events)
    }

    fn bind(&mut self, events: &[KeyEvent], action: Action) {
        self.root.bind(events, action);
    }
}

impl Default for KeyTrie {
    fn default() -> Self {
        KeyTrie {
            root: KeyTrieNode {
                action: None,
                map: FxHashMap::default(),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct KeyTrieNode {
    action: Option<Action>,
    map: FxHashMap<KeyEvent, KeyTrieNode>,
}

impl KeyTrieNode {
    fn bind(&mut self, events: &[KeyEvent], new_action: Action) {
        match events.first() {
            Some(event) => match self.map.get_mut(event) {
                Some(node) => {
                    node.bind(&events[1..], new_action);
                }
                None => {
                    let mut node = KeyTrieNode {
                        action: None,
                        map: FxHashMap::default(),
                    };
                    node.bind(&events[1..], new_action);
                    self.map.insert(event.clone(), node);
                }
            },
            None => self.action = Some(new_action),
        }
    }

    fn get(&self, events: &[KeyEvent]) -> KeymapResult {
        if events.is_empty() {
            return match &self.action {
                Some(action) => KeymapResult::Matched(action.clone()),
                None => {
                    if !self.map.is_empty() {
                        KeymapResult::Pending
                    } else {
                        KeymapResult::NotFound
                    }
                }
            };
        }

        if let Some(node) = self.map.get(&events[0]) {
            return node.get(&events[1..]);
        }

        KeymapResult::NotFound
    }

    fn find_bound_key(&self, name: &str) -> Option<Vec<KeyEvent>> {
        if let Some(ref action) = self.action {
            if action.name() == name {
                return Some(vec![]);
            }
        }

        for (ev, n) in &self.map {
            if let Some(mut events) = n.find_bound_key(name) {
                events.insert(0, ev.clone());
                return Some(events);
            }
        }

        None
    }
}
