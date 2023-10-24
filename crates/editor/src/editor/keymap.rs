use std::collections::HashMap;

use sanedit_messages::{try_parse_keyevents, KeyEvent};

use crate::actions::{completion, cursors, editor, movement, prompt, search, text, view, Action};

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
             "ctrl+c", editor::quit,
             "ctrl+s", text::save,
             "up", movement::prev_line,
             "down", movement::next_line,
             "left", movement::prev_grapheme,
             "right", movement::next_grapheme,
             "backspace", text::remove_grapheme_before_cursor,
             "delete", text::remove_grapheme_after_cursor,

             "alt+b", movement::end_of_buffer,
             "alt+B", movement::start_of_buffer,

             "alt+l", movement::end_of_line,
             "alt+L", movement::start_of_line,

             // "alt+l", Action::next_visual_line,
             // "alt+L", Action::prev_visual_line,

             "alt+w", movement::next_word_start,
             "alt+W", movement::prev_word_start,

             "alt+e", movement::next_word_end,
             "alt+E", movement::prev_word_end,

             "alt+p", movement::next_paragraph,
             "alt+P", movement::prev_paragraph,

             "alt+s", view::scroll_down,
             "alt+S", view::scroll_up,

             "ctrl+o", prompt::open_file,
             "ctrl+f", search::forward,
             "ctrl+g", search::backward,
             "ctrl+h", search::clear_matches,

             "esc", cursors::remove_secondary,
             "alt+down", cursors::new_next_line,
             "alt+up", cursors::new_prev_line,
             "ctrl+d", cursors::new_to_next_search_match,
             "ctrl+l", cursors::new_to_all_search_matches,

             "alt+n", search::next_match,
             "alt+N", search::prev_match,
             "alt+m", movement::goto_matching_pair,

             "alt+k", completion::complete,

             "ctrl+z", text::undo,
             "ctrl+r", text::redo,
             "ctrl+b", cursors::start_selection,

             "alt+r", prompt::shell_command,
        );

        map
    }

    pub fn prompt() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", prompt::close,
             "backspace", prompt::remove_grapheme_before_cursor,
             "left", prompt::prev_grapheme,
             "right", prompt::next_grapheme,
             "tab", prompt::next_completion,
             "btab", prompt::prev_completion,
             "enter", prompt::confirm,
             "up", prompt::history_prev,
             "down", prompt::history_next,
        );

        map
    }

    pub fn search() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        #[rustfmt::skip]
        map!(map,
             "ctrl+c", search::close,
             "backspace", search::remove_grapheme_before_cursor,
             "left", search::prev_grapheme,
             "right", search::next_grapheme,
             "enter", search::confirm,
             "ctrl+enter", search::confirm_all,
             "alt+enter", search::confirm_all,
             "up", search::history_prev,
             "down", search::history_next,

             // "ctrl+r", search::toggle_regex,
             // "ctrl+s", search::toggle_select,
        );

        map
    }

    pub fn completion() -> Keymap {
        let mut map = Keymap {
            root: KeyTrie::default(),
        };

        // #[rustfmt::skip]
        // map!(map,
        // );

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
