use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustc_hash::FxHashMap;
use sanedit_buffer::ReadOnlyPieceTree;
use sanedit_parser::Annotation;
use tokio::sync::broadcast;

use crate::{
    common::dirs::ConfigDirectory,
    editor::buffers::{BufferId, Filetype},
};

use self::grammar::Grammar;

mod grammar;

#[derive(Debug)]
pub(crate) struct Syntaxes {
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
        let peg = {
            let mut conf = self.filetype_dir.clone();
            conf.push(ft.as_str());
            conf.push(format!("{}.peg", ft.as_str()));
            conf
        };
        let grammar = Grammar::from_path(&peg)?;
        let syntax = Syntax {
            grammar: Arc::new(grammar),
        };
        self.syntaxes.insert(ft.clone(), syntax.clone());
        Ok(syntax)
    }
}

impl Default for Syntaxes {
    fn default() -> Self {
        let ft_dir = ConfigDirectory::default().filetype_dir();
        Syntaxes {
            filetype_dir: ft_dir,
            syntaxes: FxHashMap::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Syntax {
    grammar: Arc<Grammar>,
}

impl Syntax {
    pub fn parse(
        &self,
        bid: BufferId,
        ropt: &ReadOnlyPieceTree,
        mut view: Range<usize>,
        kill: broadcast::Receiver<()>,
    ) -> anyhow::Result<SyntaxParseResult> {
        const COMPLETION_ANNOTATION: &str = "completion";
        const HIGHLIGHT_ANNOTATION: &str = "highlight";
        // TODO try to match these to newlines
        // const HORIZON_TOP: usize = 1024 * 32;
        // const HORIZON_BOTTOM: usize = 1024;
        const HORIZON_TOP: usize = 0;
        const HORIZON_BOTTOM: usize = 0;

        // view.start = view.start.saturating_sub(HORIZON_TOP);
        // view.end = min(ropt.len(), view.end + HORIZON_BOTTOM);

        let start = view.start;
        let slice = ropt.slice(view);

        let captures = self.grammar.parse(&slice, kill)?;
        let spans: Vec<Span> = captures
            .into_iter()
            .map(|cap| {
                let name = self.grammar.label_for(cap.id());
                let mut range = cap.range();
                range.start += start;
                range.end += start;

                let anns = self.grammar.annotations_for(cap.id());
                let completion = anns.iter().any(|ann| match ann {
                    Annotation::Other(name, _spec) => name == COMPLETION_ANNOTATION,
                    _ => false,
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
            bid,
            highlights: spans,
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct SyntaxParseResult {
    pub(crate) bid: BufferId,
    pub(crate) highlights: Vec<Span>,
}

#[derive(Debug)]
pub(crate) struct Span {
    name: String,
    range: Range<usize>,
    completion: bool,
    highlight: bool,
}

impl Span {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Wether this span is completion candidate
    pub fn is_completion(&self) -> bool {
        self.completion
    }

    /// Wether this span should be highlighted or not
    pub fn highlight(&self) -> bool {
        self.highlight || !self.completion
    }
}
