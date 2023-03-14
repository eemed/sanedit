use super::{Prompt, search::Search};

/// Overlays may eat input away before it reaches the default window handling.
/// They can also provide custom key bindings.
///
/// The idea is to create a stack of overlays that may consume the input. This
/// way we can control where input is handled in an easy way.
#[derive(Debug)]
pub(crate) enum Overlay {
    Prompt(Prompt),
    Search(Search),
}

impl Overlay {
}
