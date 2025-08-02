use crate::{ParseError, Parser};
use std::sync::Arc;

/// Load languages on demand
pub trait LanguageLoader: std::fmt::Debug + Send + Sync {
    /// Get language parser for another language
    /// None if not found or failed
    fn load(&self, language: &str) -> Result<Arc<Parser>, ParseError>;
}
