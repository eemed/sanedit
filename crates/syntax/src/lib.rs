pub(crate) mod grammar;

mod error;
mod finder;
mod glob;
mod parsing_machine;
mod regex;
mod source;

pub use error::ParseError;
use grammar::Rules;
pub use source::{ByteSource, SliceSource};

pub use finder::{Finder, FinderIter, FinderIterRev, FinderRev};
pub use glob::Glob;
pub use glob::GlobError;
pub use grammar::Annotation;
pub use parsing_machine::*;
pub use regex::{Regex, RegexError, RegexRules};

/// Wrapper around JIT and Interpreted parser.
/// Try to create JIT and fallback to interpreted if necessary.
#[derive(Debug)]
pub enum Parser {
    Interpreted(ParsingMachine),
    Jit(Jit),
}

impl Parser {
    pub fn new<R: std::io::Read>(read: R) -> Result<Parser, ParseError> {
        let rules = Rules::parse(read).map_err(|err| ParseError::Grammar(err.to_string()))?;
        Self::from_rules(rules)
    }

    pub(crate) fn from_rules(rules: Rules) -> Result<Parser, ParseError> {
        let compiler = Compiler::new(&rules);
        let ops = compiler
            .compile()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;

        Self::from_ops(rules, ops)
    }

    pub(crate) fn from_ops(rules: Rules, ops: Program) -> Result<Parser, ParseError> {
        if !Jit::is_available() {
            let parser = ParsingMachine {
                rules,
                program: ops,
            };
            return Ok(Parser::Interpreted(parser));
        }

        let (program, start) = Jit::compile(&ops);
        let jit = Jit {
            rules,
            ops,
            program,
            start,
        };
        Ok(Parser::Jit(jit))
    }

    pub(crate) fn from_rules_unanchored(rules: Rules) -> Result<Parser, ParseError> {
        let compiler = Compiler::new(&rules);
        let program = compiler
            .compile_unanchored()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;
        Self::from_ops(rules, program)
    }

    pub fn parse<B: ByteSource>(&self, bytes: B) -> Result<CaptureList, ParseError> {
        match self {
            Parser::Interpreted(parsing_machine) => parsing_machine.parse(bytes),
            Parser::Jit(jit) => jit.parse(bytes),
        }
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        match self {
            Parser::Interpreted(parsing_machine) => parsing_machine.label_for(id),
            Parser::Jit(jit) => jit.label_for(id),
        }
    }

    pub fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        match self {
            Parser::Interpreted(parsing_machine) => parsing_machine.annotations_for(id),
            Parser::Jit(jit) => jit.annotations_for(id),
        }
    }

    /// Try to match text multiple times. Skips errors and yields an element only when part of the text matches
    pub fn captures<'a, B: ByteSource>(&'a self, source: B) -> CaptureIter<'a, B> {
        match self {
            Parser::Interpreted(parsing_machine) => parsing_machine.captures(source),
            Parser::Jit(jit) => jit.captures(source),
        }
    }

    pub fn program(&self) -> &Program {
        match self {
            Parser::Interpreted(parsing_machine) => parsing_machine.program(),
            Parser::Jit(jit) => &jit.ops,
        }
    }
}
