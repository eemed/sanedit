use crate::editor::keymap::Keymap;

use super::{search::Search, Prompt};

/// Overlays may eat input away before it reaches the default window handling.
/// They can also provide custom key bindings.
///
/// The idea is to create a stack of overlays that may consume the input. This
/// way we can control where input is handled in an easy way.
#[derive(Debug)]
pub(crate) enum Layer {
    Prompt(Prompt),
    Search(Search),
}

impl Layer {
    pub fn keymap(&self) -> Option<&Keymap> {
        match self {
            Layer::Prompt(p) => Some(p.keymap()),
            Layer::Search(s) => Some(s.keymap()),
        }
    }

    pub fn handle_insert(&mut self, text: &str) -> bool {
        match self {
            Layer::Prompt(p) => {
                p.insert_at_cursor(text);
                true
            }
            Layer::Search(s) => {
                s.prompt_mut().insert_at_cursor(text);
                true
            }
        }
    }
}
