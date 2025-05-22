use rustc_hash::FxHashMap;
use sanedit_messages::key::KeyEvent;

use crate::actions::Action;

use super::windows::Focus;
use super::windows::Mode;
use super::Map;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub(crate) struct LayerKey {
    pub(crate) focus: Focus,
    pub(crate) mode: Mode,
}

impl LayerKey {
    pub fn insert(&self) -> bool {
        self.focus != Focus::Window || self.mode == Mode::Insert
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Layer {
    root: KeyTrie,

    /// Action to run when this layer is entered
    pub(crate) on_enter: Option<Action>,

    /// Action to run when this layer is left
    pub(crate) on_leave: Option<Action>,

    /// if no keybinding found whether to fallthrough to the next layer
    pub(crate) fallthrough: Option<LayerKey>,
}

impl Layer {
    pub fn new() -> Layer {
        Layer {
            on_enter: None,
            on_leave: None,
            root: KeyTrie::default(),
            fallthrough: None,
        }
    }

    pub fn bind(&mut self, events: &[KeyEvent], action: &Action) {
        self.root.bind(events, action)
    }

    pub fn get(&self, events: &[KeyEvent]) -> KeytrieResult {
        self.root.get(events)
    }

    pub fn find_bound_key(&self, name: &str) -> Option<Vec<KeyEvent>> {
        self.root.root.find_bound_key(name)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Keymaps {
    layers: Map<LayerKey, Layer>,
}

impl Keymaps {
    pub fn get_layer(&self, key: &LayerKey) -> Option<&Layer> {
        self.layers.get(key)
    }

    pub fn get(&self, key: &LayerKey, events: &[KeyEvent]) -> KeymapResult {
        let mut key = *key;
        let mut layer = &self.layers[&key];
        let mut result = layer.get(events);

        while matches!(result, KeytrieResult::NotFound) {
            // No fallthrough or no new layer to fallto
            match &layer.fallthrough {
                Some(l) => {
                    key = *l;
                    layer = &self.layers[l];
                    result = layer.get(events);
                }
                None => {
                    if key.insert() {
                        return KeymapResult::Insert;
                    } else {
                        return KeymapResult::Discard;
                    }
                }
            }
        }

        match result {
            KeytrieResult::Matched(action) => KeymapResult::Matched(action),
            KeytrieResult::Pending(action) => KeymapResult::Pending(action),
            KeytrieResult::NotFound => unreachable!(),
        }
    }

    pub fn insert(&mut self, key: LayerKey, layer: Layer) {
        self.layers.insert(key, layer);
    }

    pub fn find_bound_key(&self, key: &LayerKey, name: &str) -> Option<Vec<KeyEvent>> {
        let mut layer = &self.layers[key];
        let mut result = layer.find_bound_key(name);

        while result.is_none() {
            let next = layer.fallthrough.as_ref()?;
            layer = &self.layers[next];
            result = layer.find_bound_key(name);
        }

        result
    }
}

pub(crate) enum KeymapResult {
    Matched(Action),
    Pending(Option<Action>),
    Insert,
    Discard,
}

#[derive(Debug)]
pub(crate) enum KeytrieResult {
    Matched(Action),
    Pending(Option<Action>),
    NotFound,
}

#[derive(Debug, Clone)]
struct KeyTrie {
    root: KeyTrieNode,
}

impl KeyTrie {
    fn get(&self, events: &[KeyEvent]) -> KeytrieResult {
        self.root.get(events)
    }

    fn bind(&mut self, events: &[KeyEvent], action: &Action) {
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
    fn bind(&mut self, events: &[KeyEvent], new_action: &Action) {
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
            None => self.action = Some(new_action.clone()),
        }
    }

    fn get(&self, events: &[KeyEvent]) -> KeytrieResult {
        if events.is_empty() {
            // If next keys exist keep in pending state and return middle
            // actions
            if !self.map.is_empty() {
                return KeytrieResult::Pending(self.action.clone());
            }

            // If bound to nothing
            if self.action.is_none() {
                return KeytrieResult::NotFound;
            }

            return KeytrieResult::Matched(self.action.clone().unwrap());
        }

        if let Some(node) = self.map.get(&events[0]) {
            return node.get(&events[1..]);
        }

        KeytrieResult::NotFound
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
