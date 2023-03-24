use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use std::collections::HashMap;

use super::Style;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Theme {
    pub(crate) name: String,
    pub(crate) styles: HashMap<String, Style>,
}

impl Theme {
    pub fn new(name: &str) -> Theme {
        Theme {
            name: name.to_string(),
            styles: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set(&mut self, name: &str, style: Style) {
        self.styles.insert(name.to_string(), style);
    }

    pub fn get(&self, name: &str) -> Option<Style> {
        self.styles.get(name).cloned()
    }
}

#[derive(Clone, Copy, Debug, EnumIter, EnumString, Display, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ThemeField {
    Default,
    Statusline,
    Selection,
    EndOfBuffer,
    Symbols,

    Info,
    Warn,
    Error,

    PromptDefault,
    PromptMessage,
    PromptUserInput,
    PromptCompletion,
    PromptCompletionSelected,

    CompletionDefault,
    CompletionSelected,
}
