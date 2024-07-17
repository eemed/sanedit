use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(crate) struct Options {
    pub prompt_completions: usize,
    pub completions: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            prompt_completions: 10,
            completions: 10,
        }
    }
}
