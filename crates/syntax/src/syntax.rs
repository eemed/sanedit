use std::{
    cmp::min,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustc_hash::FxHashMap;
use sanedit_buffer::PieceTreeView;
use sanedit_core::{BufferRange, Filetype};
use sanedit_server::Kill;

use crate::Annotation;

mod grammar;
pub use grammar::*;

#[derive(Debug)]
pub struct Syntaxes {
    filetype_dir: PathBuf,
    syntaxes: FxHashMap<Filetype, Syntax>,
}

impl Syntaxes {
    pub fn new(ft_dir: &Path) -> Syntaxes {
        Syntaxes {
            filetype_dir: ft_dir.into(),
            syntaxes: FxHashMap::default(),
        }
    }

    pub fn get(&mut self, ft: &Filetype) -> anyhow::Result<Syntax> {
        match self.syntaxes.get(ft) {
            Some(s) => Ok(s.clone()),
            None => self.load(ft),
        }
    }

    pub fn load(&mut self, ft: &Filetype) -> anyhow::Result<Syntax> {
        let peg = self
            .filetype_dir
            .join(ft.as_str())
            .join(format!("{}.peg", ft.as_str()));
        let grammar = Grammar::from_path(&peg)?;
        let syntax = Syntax {
            grammar: Arc::new(grammar),
        };
        self.syntaxes.insert(ft.clone(), syntax.clone());
        Ok(syntax)
    }
}

#[derive(Debug, Clone)]
pub struct Syntax {
    grammar: Arc<Grammar>,
}

impl Syntax {
    pub fn parse(
        &self,
        pt: &PieceTreeView,
        mut view: Range<u64>,
        kill: Kill,
    ) -> anyhow::Result<SyntaxParseResult> {
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

        let captures = self.grammar.parse(&slice, kill)?;
        let spans: Vec<Span> = captures
            .into_iter()
            .map(|cap| {
                let name = self.grammar.label_for(cap.id());
                let mut range = cap.range();
                range.start += start;
                range.end += start;

                let anns = self.grammar.annotations_for(cap.id());
                let completion = anns.iter().find_map(|ann| match ann {
                    Annotation::Other(aname, cname) if aname == COMPLETION_ANNOTATION => {
                        let completion = cname.clone().unwrap_or(name.into());
                        Some(completion)
                    }
                    _ => None,
                });
                let hl = anns.iter().any(|ann| match ann {
                    Annotation::Other(name, _spec) => name == HIGHLIGHT_ANNOTATION,
                    _ => false,
                });

                Span {
                    highlight: hl,
                    completion,
                    name: name.into(),
                    range,
                }
            })
            .collect();

        Ok(SyntaxParseResult {
            buffer_range: view,
            highlights: spans,
        })
    }
}

#[derive(Debug, Default)]
pub struct SyntaxParseResult {
    pub buffer_range: BufferRange,
    pub highlights: Vec<Span>,
}

#[derive(Debug)]
pub struct Span {
    name: String,
    range: Range<u64>,
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

    pub fn range(&self) -> Range<u64> {
        self.range.clone()
    }

    /// Wether this span is completion candidate
    pub fn is_completion(&self) -> bool {
        self.completion.is_some()
    }

    pub fn completion_category(&self) -> Option<&str> {
        self.completion.as_ref().map(|s| s.as_str())
    }

    /// Wether this span should be highlighted or not
    pub fn highlight(&self) -> bool {
        self.highlight || !self.is_completion()
    }

    pub fn extend_by(&mut self, i: u64) {
        self.range.end += i;
    }

    pub fn shrink_by(&mut self, i: u64) {
        self.range.end = self.range.end.saturating_sub(i);
    }

    pub fn add_offset(&mut self, i: i128) {
        let neg = i.is_negative();
        let amount = i.abs() as u64;
        if neg {
            self.range.start = self.range.start.saturating_sub(amount);
            self.range.end = self.range.end.saturating_sub(amount);
        } else {
            self.range.start += amount;
            self.range.end += amount;
        }
    }
}
