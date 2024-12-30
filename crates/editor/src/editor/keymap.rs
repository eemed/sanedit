use std::cell::RefCell;

use rustc_hash::FxHashMap;
use sanedit_messages::key::KeyEvent;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;

use crate::actions::Action;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AsRefStr, EnumIter)]
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
pub(crate) struct Layer {
    name: String,

    root: KeyTrie,

    /// if no keybinding found whether to fallthrough to the next layer
    pub(crate) fallthrough: Option<String>,

    /// If no keybindging found whether to insert text or discard it
    /// will do nothing if fallthrough is enabled
    pub(crate) discard: bool,
}

impl Layer {
    pub fn new(name: &str) -> Layer {
        Layer {
            name: name.into(),
            root: KeyTrie::default(),
            fallthrough: None,
            discard: false,
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
    layer: RefCell<usize>,
    layers: Vec<Layer>,
}

impl Keymaps {
    pub fn layer(&self) -> &str {
        let current = *self.layer.borrow();
        &self.layers[current].name
    }

    pub fn goto_layer(&self, name: &str) {
        if let Some(pos) = self.layers.iter().position(|layer| layer.name == name) {
            *self.layer.borrow_mut() = pos;
        }
    }

    pub fn goto(&self, kind: KeymapKind) {
        if let Some(pos) = self
            .layers
            .iter()
            .position(|layer| layer.name == kind.as_ref())
        {
            *self.layer.borrow_mut() = pos;
        }
    }

    pub fn get(&self, events: &[KeyEvent]) -> KeymapResult {
        let mut current = *self.layer.borrow();
        let mut layer = &self.layers[current];
        let mut result = layer.get(events);

        while matches!(result, KeytrieResult::NotFound) {
            // No fallthrough or no new layer to fallto
            match &layer.fallthrough {
                Some(l) => {
                    current = self.layers.iter().position(|lay| &lay.name == l).unwrap();
                    layer = &self.layers[current];
                    result = layer.get(events);
                }
                None => {
                    if layer.discard {
                        return KeymapResult::Discard;
                    } else {
                        return KeymapResult::Insert;
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

    pub fn push(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    pub fn find_bound_key(&self, name: &str) -> Option<Vec<KeyEvent>> {
        let mut current = *self.layer.borrow();
        let mut layer = &self.layers[current];
        let mut result = layer.find_bound_key(name);

        while result.is_none() {
            let next = layer.fallthrough.as_ref()?;
            current = self.layers.iter().position(|lay| &lay.name == next)?;
            layer = &self.layers[current];
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
