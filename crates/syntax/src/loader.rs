use crate::{ParseError, Parser};

/// Load languages on demand
pub trait LanguageLoader: std::fmt::Debug + Send + Sync {
    /// Get language parser for another language
    /// None if not found or failed
    fn load(&mut self, language: &str) -> Result<Parser, ParseError>;
}
