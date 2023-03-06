#[derive(Debug, Clone)]
pub enum Ast {
    Seq(Box<Ast>, Box<Ast>),
    Alt(Box<Ast>, Box<Ast>),
    Char(char),
    Star(Box<Ast>, bool),
    Question(Box<Ast>, bool),
    Plus(Box<Ast>, bool),
}
