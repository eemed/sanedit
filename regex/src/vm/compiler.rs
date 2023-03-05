use crate::regex::Ast;

use super::{
    inst::{Inst, InstPtr},
    program::Program,
};

pub(crate) struct Frag {
    start: InstPtr,
    ends: Vec<InstPtr>,
}

pub(crate) struct Compiler {
    insts: Vec<Inst>,
}

impl Compiler {
    /// Compile AST to a program that can be executed on the vm
    pub fn compile(ast: &Ast) -> Program {
        let mut compiler = Compiler::new();
        let frag = compiler.expr(ast);
        let n = compiler.push_inst(Inst::Match);
        todo!()
    }

    fn new() -> Compiler {
        Compiler { insts: Vec::new() }
    }

    fn expr(&mut self, ast: &Ast) -> Frag {
        match ast {
            Ast::Seq(seq) => self.seq(seq),
            Ast::Alt(alt) => self.alt(alt),
            Ast::Char(ch) => self.char(*ch),
            Ast::Star(ast, lazy) => self.star(ast),
            Ast::Question(ast, lazy) => self.question(ast),
            Ast::Plus(ast, lazy) => self.plus(ast),
        }
    }

    fn seq(&mut self, asts: &[Ast]) -> Frag {
        let asts = asts.iter();
        let mut first = self.expr(asts.next().unwrap());
        for ast in asts {
            self.expr(ast);
            // e1e2
            //     codes for e1
            //     codes for e2
        }

        first
    }

    fn alt(&mut self, asts: &[Ast]) -> Frag {
        let asts = asts.iter();
        let mut frag = self.expr(asts.next().unwrap());
        for ast in asts {
            // Split for each ast
            // split L1, L2
            // L1: codes for e1
            //     jmp L3
            // L2: codes for e2
            // L3:
        }
        todo!()
    }

    fn char(&mut self, ch: char) -> Frag {
        let mut buf = [0u8; 4];
        ch.encode_utf8(&mut buf);
        let first = self.push_inst(Inst::Byte(buf[0]));
        for i in 1..ch.len_utf8() {
            self.push_inst(Inst::Byte(buf[i]));
        }
        // codes for e1
        // codes for e2

        Frag {
            start: first,
            ends: Vec::new(),
        }
    }

    fn star(&mut self, ast: &Ast) -> Frag {
        let frag = self.expr(ast);
        // L1: split L2, L3
        // L2: codes for e
        // jmp L1
        // L3:
        todo!()
    }

    fn question(&mut self, ast: &Ast) -> Frag {
        // split L1, L2
        // L1: codes for e
        // L2:
        todo!()
    }

    fn plus(&mut self, ast: &Ast) -> Frag {
        // L1: codes for e
        // split L1, L3
        // L3:
        todo!()
    }

    fn push_inst(&mut self, inst: Inst) -> InstPtr {
        let n = self.insts.len();
        self.insts.push(inst);
        n
    }
}
