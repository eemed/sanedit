use crate::common::char::DisplayOptions;

#[derive(Debug)]
pub(crate) struct WindowOptions {
    pub display: DisplayOptions,
    pub prompt_completions: usize,
}

impl Default for WindowOptions {
    fn default() -> Self {
        WindowOptions {
            display: DisplayOptions::default(),
            prompt_completions: 10,
        }
    }
}
