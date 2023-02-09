/// Used to determine where to send input, and what to redraw
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Normal,
    Prompt,
}
