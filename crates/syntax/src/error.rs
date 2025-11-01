use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read source: {0}")]
    SourceReadError(#[from] std::io::Error),

    #[error("No language found: {0}")]
    NoLanguage(String),

    #[error("No rules found")]
    NoRules,

    #[error("Failed to parse grammar: {0}")]
    Grammar(String),

    #[error("Failed to preprocess rules: {0}")]
    Preprocess(String),

    #[error("Failed to parse: {0}")]
    Parse(String),

    #[error("Failed to parse text does not match rules")]
    ParsingFailed,

    #[error("JIT is unsupported")]
    JitUnsupported,

    #[error("Stopped by user")]
    UserStop,
}
