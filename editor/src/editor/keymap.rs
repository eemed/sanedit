use std::collections::HashMap;

use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use super::mode::Mode;
use crate::{
    actions::Action,
    model::{key::KeyPress, KEY_PRESS_SEPARATOR},
};


// Window should hold hashmap<Name, Keymap>
// and events vec<keypress>, not sure where to place these yet

#[derive(Debug, Clone)]
pub(crate) struct Keymap {
    root: KeyTrie,
}

impl Keymap {
    fn get(&mut self, events: &[KeyPress]) -> KeymapResult {
        self.root.get(events)
    }

    fn bind(&mut self, events: &[KeyPress], action: Action) {
        self.root.bind(events, action);
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap {
            root: KeyTrie::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum KeymapResult {
    Matched { action: Action },
    Pending,
    NotFound,
}

#[derive(Debug, Clone)]
struct KeyTrie {
    root: KeyTrieNode,
}

impl KeyTrie {
    fn get(&self, events: &[KeyPress]) -> KeymapResult {
        self.root.get(events)
    }

    fn bind(&mut self, events: &[KeyPress], action: Action) {
        log::debug!(
            "bind: {}, \t{}",
            events
                .iter()
                .map(|key| format!("{}", key))
                .collect::<Vec<String>>()
                .join(KEY_PRESS_SEPARATOR),
            action
        );
        self.root.bind(events, action);
    }
}

impl Default for KeyTrie {
    fn default() -> Self {
        KeyTrie {
            root: KeyTrieNode::Node {
                map: HashMap::new(),
            },
        }
    }
}

#[derive(Debug, Clone)]
enum KeyTrieNode {
    Leaf { action: Action },
    Node { map: HashMap<KeyPress, KeyTrieNode> },
}

impl KeyTrieNode {
    fn bind(&mut self, events: &[KeyPress], new_action: Action) {
        use KeyTrieNode::*;
        match self {
            Leaf { action: _ } => {
                if events.first().is_some() {
                    let mut node = Node {
                        map: HashMap::new(),
                    };
                    node.bind(&events[1..], new_action);
                    *self = node;
                    return;
                }

                // There is no more key events.
                *self = Leaf { action: new_action };
            }
            Node { map } => {
                if let Some(event) = events.first() {
                    if let Some(node) = map.get_mut(event) {
                        node.bind(&events[1..], new_action);
                    } else {
                        let mut node = Node {
                            map: HashMap::new(),
                        };
                        node.bind(&events[1..], new_action);
                        map.insert(event.clone(), node);
                    }
                    return;
                }

                *self = Leaf { action: new_action };
            }
        }
    }

    fn get(&self, events: &[KeyPress]) -> KeymapResult {
        match self {
            KeyTrieNode::Leaf { action } => {
                if events.is_empty() {
                    return KeymapResult::Matched {
                        action: action.clone(),
                    };
                }

                KeymapResult::NotFound
            }
            KeyTrieNode::Node { map } => {
                if events.is_empty() && !map.is_empty() {
                    return KeymapResult::Pending;
                }

                if events.is_empty() {
                    return KeymapResult::NotFound;
                }

                if let Some(node) = map.get(&events[0]) {
                    return node.get(&events[1..]);
                }

                KeymapResult::NotFound
            }
        }
    }
}
