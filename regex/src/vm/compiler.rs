use crate::regex::Ast;

use super::program::Program;

pub(crate) struct Compiler {}

impl Compiler {
    /// Compile AST to a program that can be executed on the vm
    pub fn compile(ast: Ast) -> Program {
        let mut compiler = Compiler::new();
        todo!()
    }

    fn new() -> Compiler {
        Compiler {}
    }
}
