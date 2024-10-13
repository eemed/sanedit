use std::path::PathBuf;

use sanedit_utils::lru::LRU;

use super::config::Config;

#[derive(Debug)]
pub(crate) struct Caches {
    pub(crate) files: LRU<PathBuf>,
}

impl Caches {
    pub fn new(config: &Config) -> Self {
        Caches {
            files: LRU::new(config.window.max_prompt_completions),
        }
    }
}
