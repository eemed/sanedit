mod default;

use rustc_hash::FxHashMap;
use sanedit_messages::KeyEvent;
use strum_macros::AsRefStr;

use crate::actions::Action;

pub(crate) use default::DefaultKeyMappings;

#[macro_export]
macro_rules! map {
    ($keymap:ident, $($mapping: expr, $action:expr),+,) => {
        use sanedit_messages::{try_parse_keyevents};
        $(
            $keymap.bind(&try_parse_keyevents($mapping).unwrap(), $action);
         )*
    }
}

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
            root: KeyTrieNode::Node {
                map: FxHashMap::default(),
            },
        }
    }
}

#[derive(Debug, Clone)]
enum KeyTrieNode {
    Leaf {
        action: Action,
    },
    Node {
        map: FxHashMap<KeyEvent, KeyTrieNode>,
    },
}

impl KeyTrieNode {
    fn bind(&mut self, events: &[KeyEvent], new_action: Action) {
        use KeyTrieNode::*;
        match self {
            Leaf { action: _ } => {
                if events.first().is_some() {
                    let mut node = Node {
                        map: FxHashMap::default(),
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
                            map: FxHashMap::default(),
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

    fn get(&self, events: &[KeyEvent]) -> KeymapResult {
        match self {
            KeyTrieNode::Leaf { action } => {
                if events.is_empty() {
                    return KeymapResult::Matched(action.clone());
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

    fn find_bound_key(&self, name: &str) -> Option<Vec<KeyEvent>> {
        match self {
            KeyTrieNode::Leaf { action } => {
                if action.name() == name {
                    Some(vec![])
                } else {
                    None
                }
            }
            KeyTrieNode::Node { map } => {
                for (ev, n) in map {
                    if let Some(mut events) = n.find_bound_key(name) {
                        events.insert(0, ev.clone());
                        return Some(events);
                    }
                }

                None
            }
        }
    }
}
