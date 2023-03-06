use std::mem;

use crate::regex::Ast;

use super::{
    inst::{Inst, InstIndex},
    program::Program,
};

pub(crate) struct Frag {
    start: InstIndex,
    ends: Vec<InstIndex>,
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
        Program {
            start: frag.start,
            insts: mem::take(&mut compiler.insts),
        }
    }

    fn new() -> Compiler {
        Compiler { insts: Vec::new() }
    }

    fn expr(&mut self, ast: &Ast) -> Frag {
        match ast {
            Ast::Seq(left, right) => self.seq(left, right),
            Ast::Alt(left, right) => self.alt(left, right),
            Ast::Char(ch) => self.char(*ch),
            Ast::Star(ast, lazy) => self.star(ast),
            Ast::Question(ast, lazy) => self.question(ast),
            Ast::Plus(ast, lazy) => self.plus(ast),
        }
    }

    fn seq(&mut self, left: &Ast, right: &Ast) -> Frag {
        // e1e2
        //     codes for e1
        //     codes for e2
        let frag = self.expr(left);
        self.expr(right);
        frag
    }

    fn alt(&mut self, left: &Ast, right: &Ast) -> Frag {
        // split L1, L2
        // L1: codes for e1
        //     jmp L3
        // L2: codes for e2
        // L3:

        let next = self.next_pos() + 1;
        let split = self.push_inst(Inst::Split(next, 0));
        let lfrag = self.expr(left);
        let jmp = self.push_inst(Inst::Jmp(0));
        let rfrag = self.expr(right);

        if let Inst::Split(_, b) = &mut self.insts[split] {
            *b = rfrag.start;
        }

        let next = self.next_pos();
        if let Inst::Jmp(a) = &mut self.insts[jmp] {
            *a = next;
        }

        lfrag
    }

    fn char(&mut self, ch: char) -> Frag {
        let mut buf = [0u8; 4];
        ch.encode_utf8(&mut buf);
        let first = self.push_inst(Inst::Byte(buf[0]));
        for i in 1..ch.len_utf8() {
            self.push_inst(Inst::Byte(buf[i]));
        }

        Frag {
            start: first,
            ends: Vec::new(),
        }
    }

    fn star(&mut self, ast: &Ast) -> Frag {
        // L1: split L2, L3
        // L2: codes for e
        // jmp L1
        // L3:
        let l1 = self.push_inst(Inst::Split(0, 0));
        let frag = self.expr(ast);
        let jmp = self.push_inst(Inst::Jmp(l1));
        let l3 = self.next_pos();
        if let Inst::Split(a, b) = &mut self.insts[l1] {
            *a = frag.start;
            *b = l3;
        }
        Frag {
            start: l1,
            ends: frag.ends,
        }
    }

    fn question(&mut self, ast: &Ast) -> Frag {
        // split L1, L2
        // L1: codes for e
        // L2:

        let pos = self.push_inst(Inst::Split(0, 0));
        let frag = self.expr(ast);
        let l2 = self.next_pos();
        if let Inst::Split(a, b) = &mut self.insts[pos] {
            *a = frag.start;
            *b = l2;
        }
        Frag {
            start: pos,
            ends: frag.ends,
        }
    }

    fn plus(&mut self, ast: &Ast) -> Frag {
        // L1: codes for e
        // split L1, L3
        // L3:

        let frag = self.expr(ast);
        let l3 = self.next_pos() + 1;
        self.push_inst(Inst::Split(frag.start, l3));
        frag
    }

    fn push_inst(&mut self, inst: Inst) -> InstIndex {
        let n = self.insts.len();
        self.insts.push(inst);
        n
    }

    fn next_pos(&self) -> InstIndex {
        self.insts.len()
    }
}

#[cfg(test)]
mod test {
    use crate::regex::Parser;

    use super::*;

    #[test]
    fn simple() {
        let regex = "a+b?c*d";
        let ast = Parser::parse(regex);
        let program = Compiler::compile(&ast);
        println!("-------- Begin program '{regex}' ---------");
        for (i, inst) in program.iter().enumerate() {
            println!("{i:02}: {inst:?}");
        }
        println!("-------- end program ---------");
    }

    #[test]
    fn alt() {
        let regex = "a|b|c";
        let ast = Parser::parse(regex);
        let program = Compiler::compile(&ast);
        println!("-------- Begin program '{regex}' ---------");
        for (i, inst) in program.iter().enumerate() {
            println!("{i:02}: {inst:?}");
        }
        println!("-------- end program ---------");
    }
}
