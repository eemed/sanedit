use std::collections::HashMap;

use thiserror::Error;

use crate::grammar::{self, Clause};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to parse grammar: {0}")]
    Grammar(String),
}

// https://arxiv.org/pdf/2005.06444.pdf
#[derive(Debug)]
pub struct PikaParser {
    rules: HashMap<String, Clause>,
}

impl PikaParser {
    pub fn new(grammar: &str) -> Result<PikaParser, ParseError> {
        match grammar::parse_rules_from_str(grammar) {
            Ok(rules) => {
                let parser = PikaParser { rules };
                Ok(parser)
            }
            Err(e) => Err(ParseError::Grammar(e.to_string())),
        }
    }
}
