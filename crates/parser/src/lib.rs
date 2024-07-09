pub(crate) mod grammar;

mod byte_reader;
mod parsing_machine;

pub use byte_reader::ByteReader;

use thiserror::Error;

pub use grammar::Annotation;
pub use parsing_machine::*;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse grammar: {0}")]
    Grammar(String),

    #[error("Failed to preprocess rules: {0}")]
    Preprocess(String),

    #[error("Failed to parse: {0}")]
    Parse(String),
}
