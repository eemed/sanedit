use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Search,
    Prompt,
    Window,
    Completion,
    Filetree,
    Locations,
}

/// Stores previously focused elements
#[derive(Default, Debug)]
pub(crate) struct FocusStack(Vec<FocusEntry>);

impl Deref for FocusStack {
    type Target = Vec<FocusEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FocusStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub(crate) struct FocusEntry {
    pub(crate) focus: Focus,
    pub(crate) keymap_layer: String,
}
