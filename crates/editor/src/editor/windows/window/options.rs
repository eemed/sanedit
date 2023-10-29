#[derive(Debug)]
pub(crate) struct Options {
    pub prompt_completions: usize,
    pub completions: usize,
    pub show_linenumbers: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            prompt_completions: 10,
            completions: 10,
            show_linenumbers: true,
        }
    }
}
