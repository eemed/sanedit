use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_buffer::ReadOnlyPieceTree;
use sanedit_parser::AST;

use crate::editor::buffers::{BufferId, Filetype};

use self::grammar::Grammar;

mod grammar;

#[derive(Debug, Default)]
pub(crate) struct Syntaxes {
    syntaxes: HashMap<Filetype, Syntax>,
}

impl Syntaxes {
    pub fn for_filetype(&mut self, ft: &Filetype, conf_dir: &Path) -> anyhow::Result<Syntax> {
        match self.syntaxes.get(ft) {
            Some(s) => Ok(s.clone()),
            None => {
                let grammar = Grammar::for_filetype(ft, conf_dir)?;
                let syntax = Syntax {
                    grammar: Arc::new(grammar),
                };
                self.syntaxes.insert(ft.clone(), syntax.clone());
                Ok(syntax)
            }
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
        let slice = ropt.slice(..);
        let ast = self.grammar.parse(&slice);
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
