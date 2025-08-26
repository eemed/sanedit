use crate::{ParseError, Parser};
use std::sync::Arc;

/// Load languages on demand
pub trait LanguageLoader: std::fmt::Debug + Send + Sync {
    /// Get language parser for another language
    /// None if not found or failed
    /// The language string can be a file path or extension or a common identifier
    fn load(&self, language: &str) -> Result<Arc<Parser>, ParseError>;
    fn get(&self, language: &str) -> Option<Arc<Parser>>;
}
