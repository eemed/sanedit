use std::str::Chars;

pub(crate) enum Ast {
    Alt(Vec<Ast>),
    Seq(Vec<Ast>),
    Group(Box<Ast>),
    Repetion { ast: Box<Ast>, kind: RepetionKind },
    Char(char),
    Range(char, char),
    Any,
}

pub(crate) enum RepetionKind {
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
    Exact(usize),
}

pub(crate) enum ParseError {}

pub(crate) struct Parser<'a> {
    chars: Chars<'a>,
    asts: Vec<Ast>,
}

impl<'a> Parser<'a> {
    pub fn new(re: &'a str) -> Parser {
        Parser {
            chars: re.chars(),
            asts: vec![],
        }
    }

    pub fn parse(&mut self) -> Result<Ast, ParseError> {
        todo!()
    }
}
