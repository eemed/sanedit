use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(crate) struct Options {
    pub max_prompt_completions: usize,
    pub max_completions: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_prompt_completions: 10,
            max_completions: 10,
        }
    }
}
