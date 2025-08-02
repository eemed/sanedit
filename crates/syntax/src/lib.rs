pub(crate) mod grammar;

mod error;
mod finder;
mod glob;
mod loader;
mod parsing_machine;
mod regex;
mod source;

pub use error::ParseError;
use grammar::Rule;
use grammar::Rules;
pub use source::{ByteSource, PTSliceSource};
use std::sync::Arc;

pub use finder::{Finder, FinderIter, FinderIterRev, FinderRev};
pub use glob::Glob;
pub use glob::GlobError;
pub use grammar::Annotation;
pub use loader::LanguageLoader;
pub use regex::{Regex, RegexError, RegexRules};

pub use parsing_machine::{Capture, CaptureID, CaptureIter, CaptureList};
pub(crate) use parsing_machine::{
    Compiler, Jit, Operation, ParsingMachine, Program, SubjectPosition,
};

/// Wrapper around JIT and Interpreted parser.
/// Try to create JIT and fallback to interpreted if necessary.
#[derive(Debug)]
pub struct Parser {
    inner: ParserKind,
    loader: Option<Arc<dyn LanguageLoader>>,
}

impl Parser {
    /// Create a new parser.
    ///
    /// NOTE: if syntax contains injected languages you should use `from_loader` instead.
    pub fn new<R: std::io::Read>(read: R) -> Result<Parser, ParseError> {
        let inner = ParserKind::new(read)?;
        Ok(Parser {
            inner,
            loader: None,
        })
    }

    /// Create a parser using a LanguageLoader.
    /// If injected languages are present, the loader will be asked to load the rules for them.
    pub fn with_loader<L: LanguageLoader + 'static, R: std::io::Read>(
        read: R,
        loader: L,
    ) -> Result<Parser, ParseError> {
        let mut parser = Self::new(read)?;
        parser.loader = Some(Arc::new(loader));
        Ok(parser)
    }

    /// Parse bytes and return the list of captures
    pub fn parse<B: ByteSource>(&self, bytes: B) -> Result<CaptureList, ParseError> {
        self.inner.parse_with_loader(bytes, self.loader.as_ref())
    }

    /// Get label for a capture
    pub fn label_for(&self, id: CaptureID) -> &str {
        self.inner.label_for(id)
    }

    /// Get annotations for a capture
    pub fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        self.inner.annotations_for(id)
    }

    /// Try to match text multiple times. Skips errors and yields an element only when part of the text matches
    pub fn captures<'a, B: ByteSource>(&'a self, source: B) -> CaptureIter<'a, B> {
        self.inner.captures(source)
    }

    /// Underlying program, useful only for printing the instrcutions
    pub fn program(&self) -> &Program {
        self.inner.program()
    }

    /// Extracts static byte sequences from a rule
    pub fn static_bytes_per_rule<F>(&self, should_extract: F) -> Vec<Vec<u8>>
    where
        F: Fn(&str, &[Annotation]) -> bool,
    {
        self.inner.static_bytes_per_rule(should_extract)
    }

    pub fn kind(&self) -> &str {
        match self.inner {
            ParserKind::Interpreted(_) => "Interpreted",
            ParserKind::Jit(_) => "JIT",
        }
    }
}

#[derive(Debug)]
pub(crate) enum ParserKind {
    Interpreted(ParsingMachine),
    Jit(Jit),
}

impl ParserKind {
    fn new<R: std::io::Read>(read: R) -> Result<ParserKind, ParseError> {
        let rules = Rules::parse(read).map_err(|err| ParseError::Grammar(err.to_string()))?;
        Self::from_rules(rules)
    }

    fn from_rules(rules: Rules) -> Result<ParserKind, ParseError> {
        let compiler = Compiler::new(&rules);
        let ops = compiler
            .compile()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;

        Self::from_ops(rules, ops)
    }

    fn from_ops(rules: Rules, ops: Program) -> Result<ParserKind, ParseError> {
        if !Jit::is_available() {
            let parser = ParsingMachine::new(rules, ops);
            return Ok(ParserKind::Interpreted(parser));
        }

        let jit = Jit::new(rules, ops);
        Ok(ParserKind::Jit(jit))
    }

    fn from_rules_unanchored(rules: Rules) -> Result<ParserKind, ParseError> {
        let compiler = Compiler::new(&rules);
        let program = compiler
            .compile_unanchored()
            .map_err(|err| ParseError::Preprocess(err.to_string()))?;
        Self::from_ops(rules, program)
    }

    fn parse<B: ByteSource>(
        &self,
        bytes: B,
    ) -> Result<CaptureList, ParseError> {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.parse(bytes),
            ParserKind::Jit(jit) => jit.parse(bytes),
        }
    }

    fn parse_with_loader<B: ByteSource>(
        &self,
        bytes: B,
        loader: Option<&Arc<dyn LanguageLoader>>,
    ) -> Result<CaptureList, ParseError> {
        let capture_list = match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.parse(bytes),
            ParserKind::Jit(jit) => jit.parse(bytes),
        }?;

        if loader.is_none() {
            return Ok(capture_list);
        }

        let loader = loader.unwrap();
        let rules = match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.rules(),
            ParserKind::Jit(jit) => jit.rules(),
        };

        let injection_ids = rules.injection_ids();
        if injection_ids.is_empty() {
            return Ok(capture_list);
        }

        enum InjectKind {
            Language,
            Place,
        }

        for (i, cap) in capture_list.iter().enumerate() {
            use Annotation::*;

            if injection_ids.contains(&cap.id) {
                let annotations = self.annotations_for(cap.id);
                let inject_ann = annotations.iter().find(|ann| matches!(ann, Inject(..)));
                let inject_lang_ann = annotations.iter().find(|ann| matches!(ann, InjectionLanguage));

                match (inject_ann, inject_lang_ann) {
                    (Some(Inject(Some(lang))), None) => {
                        let parser = loader.load(&lang);
                    }
                    (Some(Inject(None)), None) => {}
                    (None, Some(InjectionLanguage)) => {}
                    _ => {}
                }
            }
        }
        todo!()
    }

    fn label_for(&self, id: CaptureID) -> &str {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.label_for(id),
            ParserKind::Jit(jit) => jit.label_for(id),
        }
    }

    fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.annotations_for(id),
            ParserKind::Jit(jit) => jit.annotations_for(id),
        }
    }

    /// Try to match text multiple times. Skips errors and yields an element only when part of the text matches
    fn captures<'a, B: ByteSource>(&'a self, source: B) -> CaptureIter<'a, B> {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.captures(source),
            ParserKind::Jit(jit) => jit.captures(source),
        }
    }

    fn program(&self) -> &Program {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.program(),
            ParserKind::Jit(jit) => jit.program(),
        }
    }

    fn static_bytes_per_rule<F>(&self, should_extract: F) -> Vec<Vec<u8>>
    where
        F: Fn(&str, &[Annotation]) -> bool,
    {
        let rules = match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.rules(),
            ParserKind::Jit(jit) => jit.rules(),
        };

        let mut byte_sequences = vec![];
        let mut stack = vec![];
        for rule in rules.iter() {
            let extract = should_extract(&rule.name, &rule.annotations);

            if extract {
                stack.push(&rule.rule);

                while let Some(current) = stack.pop() {
                    use Rule::*;
                    match current {
                        NotFollowedBy(rule) | FollowedBy(rule) | OneOrMore(rule)
                        | ZeroOrMore(rule) | Optional(rule) => stack.push(rule),
                        Choice(vec) | Sequence(vec) => vec.iter().for_each(|r| stack.push(r)),
                        ByteSequence(vec) => byte_sequences.push(vec.clone()),
                        _ => {}
                    }
                }
            }
        }

        byte_sequences
    }
}
