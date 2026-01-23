use std::{
    cmp::{min, Ordering},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{movement, BufferRange, Detect, Directory, Language, Range};
use sanedit_messages::redraw::{Color, Style};
use sanedit_server::Kill;
use sanedit_utils::sorted_vec::SortedVec;

use std::fs::File;

use anyhow::{anyhow, bail};
use sanedit_syntax::{Annotation, Capture, Captures, LanguageLoader, Parser, PieceTreeSliceSource};

use crate::common::Choice;

use super::Map;

pub const SYNTAX_FILE: &str = "syntax.peg";

#[derive(Debug)]
pub struct Syntaxes {
    syntaxes: Arc<Mutex<Map<Language, Syntax>>>,
}

impl Syntaxes {
    pub fn new() -> Syntaxes {
        Syntaxes {
            syntaxes: Arc::new(Mutex::new(Map::default())),
        }
    }

    pub fn get(&mut self, ft: &Language) -> anyhow::Result<Syntax> {
        let syns = self
            .syntaxes
            .lock()
            .map_err(|_| anyhow!("Syntax locking failed"))?;
        syns.get(ft).cloned().ok_or(anyhow!("Syntax not loaded"))
    }

    pub fn contains_key(&self, ft: &Language) -> bool {
        match self.syntaxes.lock() {
            Ok(guard) => guard.contains_key(ft),
            Err(e) => {
                log::error!("Syntax locking failed: {e}");
                false
            }
        }
    }

    pub fn loader(&self, config_dir: Directory, detect: Arc<Map<String, Detect>>) -> SyntaxLoader {
        SyntaxLoader {
            dir: config_dir,
            global: self.syntaxes.clone(),
            detect,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SyntaxLoader {
    dir: Directory,
    global: Arc<Mutex<Map<Language, Syntax>>>,
    detect: Arc<Map<String, Detect>>,
    // TODO
    // there is a bug here if syntaxes are reloaded while the syntax loader is active
    // It may return wrong rule indices for captures.
    // If old syntax was used to parse and new syntax is used to reference the rules
    //
    // Not sure if worth fixing, probably will never happen
}

impl SyntaxLoader {
    pub fn load_language(&self, lang: &Language, reload: bool) {
        if let Ok(path) = self.path(lang.as_str()) {
            let result = if reload {
                self.reload_path(lang, &path)
            } else {
                self.load_path(lang, &path)
            };
            if let Err(e) = result {
                log::error!("Failed to load syntax for {}: {e}", lang.as_str());
            }
        }
    }

    pub fn reload_path(&self, ft: &Language, path: &Path) -> anyhow::Result<()> {
        let syntax = Syntax::from_path(path, self.clone())?;
        let mut syns = self
            .global
            .lock()
            .map_err(|_| anyhow!("Syntax locking failed"))?;
        syns.insert(ft.clone(), syntax);
        Ok(())
    }

    pub fn load_path(&self, ft: &Language, path: &Path) -> anyhow::Result<()> {
        let mut syns = self
            .global
            .lock()
            .map_err(|_| anyhow!("Syntax locking failed"))?;
        if syns.contains_key(ft) {
            return Ok(());
        }

        let syntax = Syntax::from_path(path, self.clone())?;
        syns.insert(ft.clone(), syntax);
        Ok(())
    }

    pub fn load_or_get(&self, lang: Language) -> Result<Syntax, sanedit_syntax::ParseError> {
        let mut syns = self
            .global
            .lock()
            .map_err(|_| sanedit_syntax::ParseError::NoLanguage(lang.as_str().into()))?;
        if let Some(syntax) = syns.get(&lang) {
            return Ok(syntax.clone());
        }

        let path = self.path(lang.as_str())?;
        match Syntax::from_path(&path, self.clone()) {
            Ok(syntax) => {
                syns.insert(lang, syntax.clone());
                Ok(syntax)
            }
            Err(e) => {
                log::error!("Failed to load syntax for {}: {e}", lang.as_str());
                Err(sanedit_syntax::ParseError::NoLanguage(lang.as_str().into()))
            }
        }
    }

    fn path(&self, lang: &str) -> Result<PathBuf, sanedit_syntax::ParseError> {
        let path = PathBuf::from(lang).join(SYNTAX_FILE);
        self.dir
            .find(&path)
            .ok_or(sanedit_syntax::ParseError::NoLanguage(lang.into()))
    }
}

impl LanguageLoader for SyntaxLoader {
    fn load(&self, language: &str) -> Result<Arc<Parser>, sanedit_syntax::ParseError> {
        let language =
            Language::determine(language, &self.detect).unwrap_or(Language::new(language));
        let syntax = self.load_or_get(language)?;
        Ok(syntax.parser)
    }

    fn get(&self, language: &str) -> Option<Arc<Parser>> {
        let language =
            Language::determine(language, &self.detect).unwrap_or(Language::new(language));
        let syns = self.global.lock().ok()?;
        syns.get(&language).map(|syn| syn.parser.clone())
    }
}

const COMPLETION_ANNOTATION: &str = "completion";
const HIGHLIGHT_ANNOTATION: &str = "highlight";
pub const HORIZON_TOP: u64 = 1024 * 8;
pub const HORIZON_TOP_MIN: u64 = 2048;
pub const HORIZON_BOTTOM: u64 = 1024 * 16;

#[derive(Debug, Clone)]
pub struct Syntax {
    parser: Arc<Parser>,
}

impl Syntax {
    pub fn from_path(peg: &Path, loader: SyntaxLoader) -> anyhow::Result<Syntax> {
        let file = match File::open(peg) {
            Ok(f) => f,
            Err(e) => bail!("Failed to read PEG file {:?}: {e}", peg),
        };

        let parser = Parser::with_loader(&file, loader)?;
        log::info!("Parsing syntax {peg:?} using {}", parser.kind());

        Ok(Syntax {
            parser: Arc::new(parser),
        })
    }

    pub fn static_completions(&self) -> Arc<Vec<Arc<Choice>>> {
        const STATIC_COMPLETION_ANNOTATION: &str = "static-completion";
        let mut static_completions = vec![];
        for (name, compls) in self
            .parser
            .static_bytes_per_rule(|_, anns| {
                anns.iter().any(|ann| match ann {
                    Annotation::Other(name, _) => name == STATIC_COMPLETION_ANNOTATION,
                    _ => false,
                })
            })
            .into_iter()
        {
            for compl in compls {
                if let Ok(compl) = String::from_utf8(compl) {
                    static_completions
                        .push(Choice::from_text_with_description(compl, name.clone()));
                }
            }
        }
        Arc::new(static_completions)
    }

    pub fn get_parser(&self) -> &Parser {
        &self.parser
    }

    pub fn parse(
        &self,
        pt: &PieceTreeSlice,
        mut view: BufferRange,
        kill: Kill,
    ) -> anyhow::Result<SyntaxResult> {
        let mut hstart = view.start.saturating_sub(HORIZON_TOP);
        let hend = min(pt.len(), view.end + HORIZON_BOTTOM);

        // Align to line start
        if hstart != 0 && hstart != view.start {
            let top = pt.slice(hstart..view.start.saturating_sub(HORIZON_TOP_MIN));
            let npos = movement::next_line_start(&top, 0);
            hstart += npos;
        }

        view.start = hstart;
        view.end = hend;

        let start = view.start;
        let slice = pt.slice(view);
        let source = PieceTreeSliceSource::with_stop(&slice, kill.into())?;
        let captures: Captures = self.parser.parse(source)?;
        let mut spans = Self::to_spans(start, &self.parser, captures.captures);

        let mut stack = captures.injections;
        while let Some((lang, captures)) = stack.pop() {
            stack.extend(captures.injections);
            let loader = self.parser.loader.as_ref().unwrap();
            let parser = loader.get(&lang).unwrap();
            let inj_spans = Self::to_spans(start, &parser, captures.captures);
            spans.merge(inj_spans)
        }

        // Apply custom colors based on text
        const COLOR_HL: &str = "color";
        // SAFETY: Will not change order
        for span in unsafe { spans.iter_mut() } {
            if span.name() != COLOR_HL {
                continue;
            }

            let text = pt.slice(span.range());
            let text = String::from(&text);
            span.style = style_from_color(text.as_str());
        }

        Ok(SyntaxResult {
            buffer_range: view,
            highlights: spans,
        })
    }

    pub fn to_spans(start: u64, parser: &Parser, captures: Vec<Capture>) -> SortedVec<Span> {
        captures
            .into_iter()
            .map(|cap| {
                let mut name = parser.label_for(cap.id());
                let mut range: BufferRange = cap.range().into();
                range.start += start;
                range.end += start;

                let anns = parser.annotations_for(cap.id());
                let completion = anns.iter().find_map(|ann| match ann {
                    Annotation::Other(aname, cname) if aname == COMPLETION_ANNOTATION => {
                        let completion = cname.clone().unwrap_or(name.into());
                        Some(completion)
                    }
                    _ => None,
                });
                let hl = anns.iter().any(|ann| match ann {
                    Annotation::Other(ann, hlname) => {
                        if ann == HIGHLIGHT_ANNOTATION {
                            if let Some(hlname) = hlname {
                                name = hlname.as_str();
                            }
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                });

                Span {
                    highlight: hl,
                    completion,
                    name: name.into(),
                    range,
                    style: None,
                }
            })
            .filter(|span| span.completion.is_some() || span.highlight)
            .collect()
    }
}

fn style_from_color(text: &str) -> Option<Style> {
    let color = Color::parse(text).ok()?;
    let fg = match color {
        Color::Black => Color::White,
        Color::White => Color::Black,
        Color::Rgb(color) => {
            let (r, g, b) = color.get();
            let brightness = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
            if brightness > 128 {
                Color::Black
            } else {
                Color::White
            }
        }
    };

    Some(Style {
        text_style: None,
        bg: Some(color),
        fg: Some(fg),
    })
}

#[derive(Debug, Default)]
pub struct SyntaxResult {
    pub buffer_range: BufferRange,
    pub highlights: SortedVec<Span>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Span {
    range: Range<u64>,
    name: String,
    completion: Option<String>,
    highlight: bool,
    style: Option<Style>,
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Span {
    fn cmp(&self, other: &Self) -> Ordering {
        let res = match self.range.start.cmp(&other.range.start) {
            Ordering::Equal => other.range.end.cmp(&self.range.end), // Larger end comes first
            other => other,
        };

        match res {
            Ordering::Equal => (&self.name, &self.completion, &self.highlight).cmp(&(
                &other.name,
                &other.completion,
                &other.highlight,
            )),
            _ => res,
        }
    }
}

impl Span {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn style(&self) -> Option<&Style> {
        self.style.as_ref()
    }

    pub fn start(&self) -> u64 {
        self.range.start
    }

    pub fn end(&self) -> u64 {
        self.range.end
    }

    pub fn range(&self) -> &BufferRange {
        &self.range
    }

    /// Wether this span is completion candidate
    pub fn is_completion(&self) -> bool {
        self.completion.is_some()
    }

    pub fn completion_category(&self) -> Option<&str> {
        self.completion.as_deref()
    }

    /// Wether this span should be highlighted or not
    pub fn highlight(&self) -> bool {
        self.highlight
    }

    pub fn extend_by(&mut self, i: u64) {
        self.range.end += i;
    }

    pub fn shrink_by(&mut self, i: u64) {
        self.range.end = self.range.end.saturating_sub(i);
    }

    pub fn add_offset(&mut self, i: i128) {
        let neg = i.is_negative();
        let amount = i.unsigned_abs() as u64;
        if neg {
            self.range.start = self.range.start.saturating_sub(amount);
            self.range.end = self.range.end.saturating_sub(amount);
        } else {
            self.range.start += amount;
            self.range.end += amount;
        }
    }
}
