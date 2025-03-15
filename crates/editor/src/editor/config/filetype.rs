use std::{cell::OnceCell, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    common::matcher::Choice,
    editor::{snippets::Snippet, Map},
};

use super::buffers;

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Default, Deserialize, DocComment)]
#[serde(default)]
pub(crate) struct FiletypeConfig {
    /// Used to comment blocks of text
    pub comment: String,

    #[serde(flatten)]
    pub buffer: buffers::BufferConfig,

    pub language_server: LSPConfig,

    pub snippets: Map<String, ConfigSnippet>,
}

impl FiletypeConfig {
    pub fn snippets_as_choices(&self) -> Vec<Arc<Choice>> {
        let mut choices = vec![];
        for (_name, snip) in &self.snippets {
            if let Some(loaded) = snip.get() {
                choices.push(Choice::from_snippet_trigger(loaded));
            }
        }

        choices
    }
}
