#[derive(Debug, Clone)]
pub enum Ast {
    Concat(Vec<Ast>),
    Or(Vec<Ast>),
    Char(char),
    Star(Box<Ast>),
    Question(Box<Ast>),
    Plus(Box<Ast>)
}
