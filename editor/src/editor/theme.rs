use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct Theme {
    pub(crate) name: String,
    pub(crate) styles: HashMap<String, CellStyle>,
}

impl Theme {
    pub fn new(name: String) -> Theme {
        Theme {
            name,
            styles: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, style: CellStyle) {
        self.styles.insert(name, style);
    }

    pub fn get(&self, name: &str) -> Option<CellStyle> {
        self.styles.get(name).cloned()
    }

    pub fn get_by(&self, id: ThemeStyle) -> Option<CellStyle> {
        self.styles.get(id.into()).cloned()
    }
}

#[derive(Clone, Copy, Debug, EnumIter, EnumString, Display, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum ThemeStyle {
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
