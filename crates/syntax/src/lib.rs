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
use std::borrow::Cow;

use std::collections::HashMap;
use std::sync::Arc;

pub use finder::{Finder, FinderIter, FinderIterRev, FinderRev};
pub use glob::Glob;
pub use glob::GlobError;
pub use grammar::Annotation;
pub use loader::LanguageLoader;
pub use regex::{Regex, RegexError, RegexRules};

pub use parsing_machine::{Capture, CaptureID, CaptureIter, CaptureList, Captures};
pub(crate) use parsing_machine::{
    Compiler, Jit, Operation, ParsingMachine, Program, SubjectPosition,
};

pub mod bench {
    pub use super::parsing_machine::Jit;
    pub use super::parsing_machine::ParsingMachine;
}

/// Get slice of bytesource efficiently
macro_rules! get_slice {
    ($bytes:expr, $range:expr) => {{
        if let Some(arr) = $bytes.as_single_chunk() {
            Cow::from(&arr[$range.start as usize..$range.end as usize])
        } else {
            let mut buf = vec![0; ($range.end - $range.start) as usize];
            let n = $bytes.copy_to($range.start, &mut buf);
            debug_assert!(n == buf.len(), "Invalid read");
            Cow::from(buf)
        }
    }};
}

/// Wrapper around JIT and Interpreted parser.
/// Try to create JIT and fallback to interpreted if necessary.
#[derive(Debug)]
pub struct Parser {
    inner: ParserKind,
    pub loader: Option<Arc<dyn LanguageLoader>>,
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
    pub fn parse<B: ByteSource>(&self, bytes: B) -> Result<Captures, ParseError> {
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
    pub fn static_bytes_per_rule<F>(&self, should_extract: F) -> HashMap<String, Vec<Vec<u8>>>
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

impl From<ParserKind> for Parser {
    fn from(value: ParserKind) -> Self {
        Parser {
            inner: value,
            loader: None,
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

    fn parse<B: ByteSource>(&self, bytes: B) -> Result<CaptureList, ParseError> {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.parse(bytes),
            ParserKind::Jit(jit) => jit.parse(bytes),
        }
    }

    fn parse_with_loader<B: ByteSource>(
        &self,
        mut bytes: B,
        loader: Option<&Arc<dyn LanguageLoader>>,
    ) -> Result<Captures, ParseError> {
        let capture_list = match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine
                .do_parse(&mut bytes, 0)
                .map(|(caps, _)| caps)
                .map_err(|err| ParseError::Parse(err.to_string()))?,
            ParserKind::Jit(jit) => jit.do_parse(&mut bytes, 0, false)?.0,
        };

        if loader.is_none() {
            return Ok(Captures {
                captures: capture_list,
                injections: vec![],
            });
        }

        self.handle_injections(bytes, loader.unwrap(), capture_list)
    }

    fn handle_injections<B: ByteSource>(
        &self,
        mut bytes: B,
        loader: &Arc<dyn LanguageLoader>,
        capture_list: CaptureList,
    ) -> Result<Captures, ParseError> {
        let rules = match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.rules(),
            ParserKind::Jit(jit) => jit.rules(),
        };

        let injection_ids = rules.injection_ids();
        if injection_ids.is_none() {
            return Ok(Captures {
                captures: capture_list,
                injections: vec![],
            });
        }

        let injection_ids = injection_ids.unwrap();
        let mut injections = vec![];
        let mut last_pos = None;
        let mut last_lang = None;
        let mut i = 0;
        let mut push_injections = |mut caps: Captures, lang: String, start: u64| {
            caps.captures.iter_mut().for_each(|cap| {
                cap.start += start;
                cap.end += start;
            });

            injections.push((lang.clone(), caps));
        };

        while i < capture_list.len() {
            use Annotation::*;
            let cap = &capture_list[i];

            if !injection_ids[cap.id] {
                i += 1;
                continue;
            }

            let annotations = self.annotations_for(cap.id);
            let inject_ann = annotations.iter().find(|ann| matches!(ann, Inject(..)));
            let inject_lang_ann = annotations
                .iter()
                .find(|ann| matches!(ann, InjectionLanguage));

            match (inject_ann, inject_lang_ann) {
                (Some(Inject(Some(lang))), None) => {
                    if let Ok(parser) = loader.load(&lang) {
                        let slice = get_slice!(bytes, cap.range());
                        let caps = parser.parse(slice)?;
                        let start = cap.range().start;
                        push_injections(caps, lang.clone(), start);
                    }
                }
                (Some(Inject(None)), None) => {
                    last_pos = Some(i);
                }
                (None, Some(InjectionLanguage)) => {
                    last_lang = Some(i);
                }
                _ => {}
            }

            if let (Some(a), Some(b)) = (last_pos, last_lang) {
                let lang_cap = &capture_list[b];
                let slice = get_slice!(bytes, lang_cap.range());

                if let Ok(lang) = std::str::from_utf8(&slice).map(String::from) {
                    if let Ok(parser) = loader.load(&lang) {
                        let pos_cap = &capture_list[a];
                        let slice = get_slice!(bytes, pos_cap.range());
                        let caps = parser.parse(slice)?;
                        let start = pos_cap.range().start;
                        push_injections(caps, lang.clone(), start);
                    }
                }

                last_pos = None;
                // last_lang = None;
            }

            i += 1;
        }

        Ok(Captures {
            captures: capture_list,
            injections,
        })
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

    #[allow(dead_code)]
    fn rules(&self) -> &Rules {
        match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.rules(),
            ParserKind::Jit(jit) => jit.rules(),
        }
    }

    fn static_bytes_per_rule<F>(&self, should_extract: F) -> HashMap<String, Vec<Vec<u8>>>
    where
        F: Fn(&str, &[Annotation]) -> bool,
    {
        let rules = match self {
            ParserKind::Interpreted(parsing_machine) => parsing_machine.rules(),
            ParserKind::Jit(jit) => jit.rules(),
        };

        let mut map: HashMap<String, Vec<Vec<u8>>> = HashMap::default();
        let mut stack = vec![];
        for rule in rules.iter() {
            let extract = should_extract(&rule.name, &rule.annotations);

            if extract {
                stack.push(&rule.rule);
                if !map.contains_key(&rule.name) {
                    map.insert(rule.name.clone(), vec![]);
                }

                while let Some(current) = stack.pop() {
                    use Rule::*;
                    match current {
                        NotFollowedBy(rule) | FollowedBy(rule) | OneOrMore(rule)
                        | ZeroOrMore(rule) | Optional(rule) => stack.push(rule),
                        Choice(vec) | Sequence(vec) => vec.iter().for_each(|r| stack.push(r)),
                        ByteSequence(vec) => {
                            map.get_mut(&rule.name).unwrap().push(vec.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        map
    }
}
