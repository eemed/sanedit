#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Search,
    Prompt,
    Window,
    Completion,
    Filetree,
}
