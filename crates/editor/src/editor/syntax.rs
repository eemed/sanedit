use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_buffer::ReadOnlyPieceTree;
use sanedit_parser::AST;

use crate::{
    common::dirs::ConfigDirectory,
    editor::buffers::{BufferId, Filetype},
};

use self::grammar::Grammar;

mod grammar;

#[derive(Debug)]
pub(crate) struct Syntaxes {
    filetype_dir: PathBuf,
    syntaxes: HashMap<Filetype, Syntax>,
}

impl Syntaxes {
    pub fn new(ft_dir: &Path) -> Syntaxes {
        Syntaxes {
            filetype_dir: ft_dir.into(),
            syntaxes: HashMap::new(),
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
            syntaxes: HashMap::new(),
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
        view: Range<usize>,
    ) -> SyntaxParseResult {
        log::info!("parsing");
        let slice = ropt.slice(..);
        let ast = self.grammar.parse(&slice);
        // log::debug!("{}", ast.print_string(&String::from(&slice)));
        let spans = ast.flatten().into_iter().map(Span::from).collect();

        SyntaxParseResult {
            bid,
            kind: ParseKind::Full,
            highlights: spans,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct SyntaxParseResult {
    pub(crate) bid: BufferId,
    pub(crate) kind: ParseKind,
    pub(crate) highlights: Vec<Span>,
}

#[derive(Debug, Default)]
pub enum ParseKind {
    #[default]
    Unparsed,
    Partial(Range<usize>),
    Full,
}

#[derive(Debug)]
pub(crate) struct Span {
    pub(crate) name: String,
    pub(crate) range: Range<usize>,
}

impl From<AST> for Span {
    fn from(ast: AST) -> Self {
        let name = ast.name().to_string();
        let range = ast.range();
        Span { name, range }
    }
}
