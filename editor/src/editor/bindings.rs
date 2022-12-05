use std::collections::HashMap;

use sanedit_messages::KeyEvent;

use crate::actions::Action;

// Window should hold hashmap<Name, Keymap>
// and events vec<keypress>, not sure where to place these yet

#[derive(Debug, Clone)]
pub(crate) struct KeyBindings {
    root: KeyTrie,
}

impl KeyBindings {
    /// Get a binding result for events.
    /// The result may be
    /// Matched => found a binding for events and its action
    /// NotFound => no binding for key combination
    /// Pending => need more input to decide
    fn get(&mut self, events: &[KeyEvent]) -> BindingResult {
        self.root.get(events)
    }

    /// Create a new binding for key combination events.
    fn bind(&mut self, events: &[KeyEvent], action: Action) {
        self.root.bind(events, action);
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            root: KeyTrie::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum BindingResult {
    Matched(Action),
    Pending,
    NotFound,
}

#[derive(Debug, Clone)]
struct KeyTrie {
    root: KeyTrieNode,
}

impl KeyTrie {
    fn get(&self, events: &[KeyEvent]) -> BindingResult {
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
                map: HashMap::new(),
            },
        }
    }
}

#[derive(Debug, Clone)]
enum KeyTrieNode {
    Leaf { action: Action },
    Node { map: HashMap<KeyEvent, KeyTrieNode> },
}

impl KeyTrieNode {
    fn bind(&mut self, events: &[KeyEvent], new_action: Action) {
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

    fn get(&self, events: &[KeyEvent]) -> BindingResult {
        match self {
            KeyTrieNode::Leaf { action } => {
                if events.is_empty() {
                    return BindingResult::Matched(action.clone());
                }

                BindingResult::NotFound
            }
            KeyTrieNode::Node { map } => {
                if events.is_empty() && !map.is_empty() {
                    return BindingResult::Pending;
                }

                if events.is_empty() {
                    return BindingResult::NotFound;
                }

                if let Some(node) = map.get(&events[0]) {
                    return node.get(&events[1..]);
                }

                BindingResult::NotFound
            }
        }
    }
}
