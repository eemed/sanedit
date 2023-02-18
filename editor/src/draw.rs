use sanedit_messages::redraw::{Prompt, Statusline, Window};

pub(crate) struct DrawContext {
    prev_prompt: Option<Prompt>,
    prev_statusline: Statusline,
    prev_window: Window,
}
