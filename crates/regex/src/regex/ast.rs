#[derive(Debug, Clone)]
pub enum Ast {
    Seq(Vec<Ast>),
    Alt(Vec<Ast>),
    Char(char),
    Star(Box<Ast>, bool),
    Question(Box<Ast>, bool),
    Plus(Box<Ast>, bool),
    Group(Box<Ast>),
}
