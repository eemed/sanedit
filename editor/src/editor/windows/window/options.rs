#[derive(Debug)]
pub(crate) struct Options {
    pub prompt_completions: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            prompt_completions: 10,
        }
    }
}
