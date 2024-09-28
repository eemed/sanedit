use std::{
    cmp::min,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use rustc_hash::FxHashMap;
use sanedit_buffer::PieceTreeView;
use sanedit_core::{BufferRange, Directory, Filetype};
use sanedit_server::Kill;

use std::fs::File;

use anyhow::{anyhow, bail};
use sanedit_buffer::{Bytes, PieceTreeSlice};
use sanedit_syntax::{Annotation, ByteReader, Parser};

#[derive(Debug)]
pub struct Syntaxes {
    filetype_dir: Directory,
    syntaxes: FxHashMap<Filetype, Syntax>,
}

impl Syntaxes {
    pub fn new(ft_dir: Directory) -> Syntaxes {
        Syntaxes {
            filetype_dir: ft_dir,
            syntaxes: FxHashMap::default(),
        }
    }

    pub fn get(&mut self, ft: &Filetype) -> anyhow::Result<Syntax> {
        self.syntaxes
            .get(ft)
            .cloned()
            .ok_or(anyhow!("Syntax not loaded"))
    }

    pub fn contains_key(&self, ft: &Filetype) -> bool {
        self.syntaxes.contains_key(ft)
    }

    pub fn load(&mut self, ft: &Filetype) -> anyhow::Result<Syntax> {
        let components = [
            PathBuf::from(ft.as_str()),
            PathBuf::from(format!("{}.peg", ft.as_str())),
        ];
        let peg = self.filetype_dir.find(&components).ok_or(anyhow!(
            "Could not find syntax for filetype {}",
            ft.as_str()
        ))?;
        let syntax = Syntax::from_path(&peg)?;
        self.syntaxes.insert(ft.clone(), syntax.clone());
        Ok(syntax)
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

        match Parser::new(file) {
            Ok(p) => Ok(Syntax {
                parser: Arc::new(p),
            }),
            Err(e) => bail!("Failed to create grammar from PEG file: {:?}: {e}", peg),
        }
    }

    pub fn parse(
        &self,
        pt: &PieceTreeView,
        mut view: Range<u64>,
        kill: Kill,
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

        let reader = PTReader { pt: slice, kill };
        let captures = self.parser.parse(reader)?;
        let spans: Vec<Span> = captures
            .into_iter()
            .map(|cap| {
                let name = self.parser.label_for(cap.id());
                let mut range = cap.range();
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

        Ok(SyntaxResult {
            buffer_range: view,
            highlights: spans,
        })
    }
}

struct PTIter<'a>(Bytes<'a>);
impl<'a> Iterator for PTIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

// TODO optimize, check performance using a bytes iterator
// and just cloning it, and limiting to a range
struct PTReader<'a> {
    pt: PieceTreeSlice<'a>,
    kill: Kill,
}

impl<'a> ByteReader for PTReader<'a> {
    type I = PTIter<'a>;

    fn len(&self) -> u64 {
        self.pt.len()
    }

    fn stop(&self) -> bool {
        self.kill.should_stop()
    }

    fn iter(&self, range: std::ops::Range<u64>) -> Self::I {
        let slice = self.pt.slice(range);
        let bytes = slice.bytes();
        PTIter(bytes)
    }
}

#[derive(Debug, Default)]
pub struct SyntaxResult {
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
        self.completion.as_deref()
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
