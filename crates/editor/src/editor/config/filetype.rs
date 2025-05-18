use std::{cell::OnceCell, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{common::matcher::Choice, editor::snippets::Snippet};

use super::buffers;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConfigSnippet {
    trigger: String,
    body: String,

    #[serde(skip)]
    loaded: OnceCell<Option<Snippet>>,
}

impl ConfigSnippet {
    pub fn get(&self) -> Option<Snippet> {
        self.loaded
            .get_or_init(|| match Snippet::new_trigger(&self.body, &self.trigger) {
                Ok(snip) => Some(snip),
                Err(e) => {
                    log::error!("Failed to load snippet: {e}");
                    None
                }
            })
            .clone()
    }
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct LSPConfig {
    /// Command to run LSP
    pub command: String,

    /// Arguments to pass onto LSP command
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct FiletypeConfig {
    /// Used to comment blocks of text
    pub comment: String,

    pub language_server: LSPConfig,

    pub buffer: buffers::BufferConfig,

    pub snippet: Vec<ConfigSnippet>,
}

impl FiletypeConfig {
    pub fn new(config_path: &Path) -> FiletypeConfig {
        match Self::try_new(config_path) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to load filetype configuration, using default instead: {e}");
                FiletypeConfig::default()
            }
        }
    }

    pub fn try_new(config_path: &Path) -> anyhow::Result<FiletypeConfig> {
        let builder = config::Config::builder().add_source(config::File::from(config_path));
        let config = builder.build()?.try_deserialize::<FiletypeConfig>()?;
        Ok(config)
    }

    pub fn snippets_as_choices(&self) -> Vec<Arc<Choice>> {
        let mut choices = vec![];
        for snip in &self.snippet {
            if let Some(loaded) = snip.get() {
                choices.push(Choice::from_snippet_trigger(loaded));
            }
        }

        choices
    }
}
