#[derive(Debug)]
pub(crate) struct WindowOptions {
    pub prompt_completions: usize,
}

impl Default for WindowOptions {
    fn default() -> Self {
        WindowOptions {
            prompt_completions: 10,
        }
    }
}
