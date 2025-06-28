use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("No rules found")]
    NoRules,

    #[error("Failed to parse grammar: {0}")]
    Grammar(String),

    #[error("Failed to preprocess rules: {0}")]
    Preprocess(String),

    #[error("Failed to parse: {0}")]
    Parse(String),

    #[error("JIT is unsupported")]
    JitUnsupported,
}
