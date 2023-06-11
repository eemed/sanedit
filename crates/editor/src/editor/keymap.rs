use std::collections::HashMap;

use sanedit_messages::{try_parse_keyevents, KeyEvent};

use crate::actions::Action;

macro_rules! map {
    ($keymap:ident, $($mapping: expr, $action:expr),+,) => {
        $(
            $keymap.bind(&try_parse_keyevents($mapping).unwrap(), $action);
         )*
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Keymap {
    root: KeyTrie,
}

impl Keymap {
    pub fn window() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", Action::quit,
             "up", Action::prev_line,
             "down", Action::next_line,
             "left", Action::prev_grapheme,
             "right", Action::next_grapheme,
             "backspace", Action::remove_grapheme_before_cursor,
             "delete", Action::remove_grapheme_after_cursor,

             "alt+b", Action::end_of_buffer,
             "alt+B", Action::start_of_buffer,

             "alt+l", Action::end_of_line,
             "alt+L", Action::start_of_line,

             // "alt+l", Action::next_visual_line,
             // "alt+L", Action::prev_visual_line,

             "alt+w", Action::next_word_start,
             "alt+W", Action::prev_word_start,

             "alt+e", Action::next_word_end,
             "alt+E", Action::prev_word_end,

             "alt+p", Action::next_paragraph,
             "alt+P", Action::prev_paragraph,

             "alt+s", Action::scroll_down,
             "alt+S", Action::scroll_up,

             "ctrl+o", Action::prompt_open_file,
             "ctrl+f", Action::search_forward,
             "ctrl+g", Action::search_backward,
             "ctrl+h", Action::search_clear_matches,

             "esc", Action::cursor_remove_secondary,
             "alt+down", Action::cursor_new_next_line,
             "alt+up", Action::cursor_new_prev_line,
             "ctrl+d", Action::cursor_new_to_next_search_match,
             "ctrl+l", Action::cursor_new_to_all_search_matches,

             "alt+n", Action::search_next_match,
             "alt+N", Action::search_prev_match,
        );

        map
    }

    pub fn prompt() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", Action::prompt_close,
             "backspace", Action::prompt_remove_grapheme_before_cursor,
             "left", Action::prompt_prev_grapheme,
             "right", Action::prompt_next_grapheme,
             "tab", Action::prompt_next_completion,
             "btab", Action::prompt_prev_completion,
             "enter", Action::prompt_confirm,
             "up", Action::prompt_history_prev,
             "down", Action::prompt_history_next,
        );

        map
    }

    pub fn search() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", Action::search_close,
             "backspace", Action::search_remove_grapheme_before_cursor,
             "left", Action::search_prev_grapheme,
             "right", Action::search_next_grapheme,
             "enter", Action::search_confirm,
             "ctrl+enter", Action::search_confirm_all,
             "alt+enter", Action::search_confirm_all,
             "up", Action::search_history_prev,
             "down", Action::search_history_next,

             // "ctrl+r", Action::search_toggle_regex,
             // "ctrl+s", Action::search_toggle_select,
        );

        map
    }

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
}
