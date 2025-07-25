use std::{cmp::min, path::Path, sync::Arc};

use rustc_hash::FxHashMap;
use sanedit_buffer::PieceTreeView;
use sanedit_core::{BufferRange, Language, Range};
use sanedit_server::Kill;
use sanedit_utils::sorted_vec::SortedVec;

use std::fs::File;

use anyhow::{anyhow, bail};
use sanedit_syntax::{Annotation, Capture, Parser, SliceSource};

#[derive(Debug)]
pub struct Syntaxes {
    syntaxes: FxHashMap<Language, Syntax>,
}

impl Syntaxes {
    pub fn new() -> Syntaxes {
        Syntaxes {
            syntaxes: FxHashMap::default(),
        }
    }

    pub fn get(&mut self, ft: &Language) -> anyhow::Result<Syntax> {
        self.syntaxes
            .get(ft)
            .cloned()
            .ok_or(anyhow!("Syntax not loaded"))
    }

    pub fn contains_key(&self, ft: &Language) -> bool {
        self.syntaxes.contains_key(ft)
    }

    pub fn reload(&mut self, ft: &Language, path: &Path) -> anyhow::Result<()> {
        let syntax = Syntax::from_path(path)?;
        self.syntaxes.insert(ft.clone(), syntax);
        Ok(())
    }

    pub fn load(&mut self, ft: &Language, path: &Path) -> anyhow::Result<()> {
        if self.syntaxes.contains_key(ft) {
            return Ok(());
        }

        let syntax = Syntax::from_path(path)?;
        self.syntaxes.insert(ft.clone(), syntax);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Syntax {
    parser: Arc<Parser>,
}

impl Syntax {
    pub fn from_path(peg: &Path) -> anyhow::Result<Syntax> {
        let file = match File::open(peg) {
            Ok(f) => f,
            Err(e) => bail!("Failed to read PEG file {:?}: {e}", peg),
        };

        let parser = Parser::new(&file)?;

        log::info!(
            "Parsing syntax {peg:?} using {}",
            if matches!(parser, Parser::Jit(..)) {
                "Jit"
            } else {
                "Interpreted"
            }
        );

        Ok(Syntax {
            parser: Arc::new(parser),
        })
    }

    pub fn parse(
        &self,
        pt: &PieceTreeView,
        mut view: BufferRange,
        _kill: Kill,
    ) -> anyhow::Result<SyntaxResult> {
        const COMPLETION_ANNOTATION: &str = "completion";
        const HIGHLIGHT_ANNOTATION: &str = "highlight";

        // TODO try to match these to newlines
        const HORIZON_TOP: u64 = 1024 * 8;
        const HORIZON_BOTTOM: u64 = 1024 * 16;
        // prev_line_start(view.start)
        // next_line_start(view.start)

        view.start = view.start.saturating_sub(HORIZON_TOP);
        view.end = min(pt.len(), view.end + HORIZON_BOTTOM);

        let start = view.start;
        let slice = pt.slice(view.clone());
        let source = SliceSource::new(&slice);
        let captures: SortedVec<Capture> = self.parser.parse(source)?.into();

        let spans: SortedVec<Span> = captures
            .into_iter()
            .map(|cap| {
                let mut name = self.parser.label_for(cap.id());
                let mut range: BufferRange = cap.range().into();
                range.start += start;
                range.end += start;

                let anns = self.parser.annotations_for(cap.id());
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
                }
            })
            .filter(|span| span.completion.is_some() || span.highlight)
            .collect();

        Ok(SyntaxResult {
            buffer_range: view,
            highlights: spans,
        })
    }
}

#[derive(Debug, Default)]
pub struct SyntaxResult {
    pub buffer_range: BufferRange,
    pub highlights: SortedVec<Span>,
}

#[derive(Debug, Ord, PartialOrd, PartialEq, Eq)]
pub struct Span {
    range: Range<u64>,
    name: String,
    completion: Option<String>,
    highlight: bool,
}

impl Span {
    pub fn name(&self) -> &str {
        &self.name
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
