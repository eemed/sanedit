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
    group: usize,
}

impl Compiler {
    /// Compile AST to a program that can be executed on the vm
    pub fn compile(ast: &Ast) -> Program {
        let mut compiler = Compiler::new();
        let frag = compiler.do_compile(ast);
        Program {
            start: frag.start,
            insts: mem::take(&mut compiler.insts),
        }
    }

    fn new() -> Compiler {
        Compiler {
            insts: Vec::new(),
            group: 0,
        }
    }

    fn do_compile(&mut self, ast: &Ast) -> Frag {
        // Add substring searching by prepending .*? insts to the start
        // 00: Split([3, 1])
        // 01: ByteRange(0..255)
        // 02: Jmp(0)
        let start = self.push_inst(Inst::Split(vec![3, 1]));
        self.push_inst(Inst::ByteRange(0..u8::MAX));
        self.push_inst(Inst::Jmp(0));

        // Extract matched range by wrapping it on save instructions
        self.group += 1;

        self.push_inst(Inst::Save(0));
        let mut frag = self.expr(ast);
        self.push_inst(Inst::Save(1));

        let _ = self.push_inst(Inst::Match);
        frag.start = start;
        frag
    }

    fn expr(&mut self, ast: &Ast) -> Frag {
        match ast {
            Ast::Seq(seq) => self.seq(seq),
            Ast::Alt(alt) => self.alt(alt),
            Ast::Char(ch) => self.char(*ch),
            Ast::Star(ast, lazy) => self.star(*lazy, ast),
            Ast::Question(ast, lazy) => self.question(*lazy, ast),
            Ast::Plus(ast, lazy) => self.plus(*lazy, ast),
            Ast::Group(ast) => self.group(ast),
            Ast::Any => self.any(),
        }
    }

    fn any(&mut self) -> Frag {
        let start = self.push_inst(Inst::ByteRange(0..u8::MAX));

        Frag {
            start,
            ends: Vec::new(),
        }
    }

    fn seq(&mut self, asts: &[Ast]) -> Frag {
        // e1e2
        //     codes for e1
        //     codes for e2
        let mut asts = asts.iter();
        let mut first = self.expr(asts.next().unwrap());
        for ast in asts {
            self.expr(ast);
        }

        first
    }

    fn alt(&mut self, asts: &[Ast]) -> Frag {
        // split L1, L2
        // L1: codes for e1
        //     jmp L3
        // L2: codes for e2
        // L3:

        let split = self.push_inst(Inst::Split(vec![]));

        let mut frags = Vec::with_capacity(asts.len());
        let mut jumps = Vec::with_capacity(asts.len());
        let mut asts = asts.iter().peekable();

        while let Some(ast) = asts.next() {
            frags.push(self.expr(ast));
            if asts.peek().is_some() {
                jumps.push(self.push_inst(Inst::Jmp(0)));
            }
        }

        if let Inst::Split(split) = &mut self.insts[split] {
            *split = frags.iter().map(|f| f.start).collect();
        }

        let end = self.next_pos();
        for jmp in jumps {
            if let Inst::Jmp(jmp) = &mut self.insts[jmp] {
                *jmp = end;
            }
        }

        Frag {
            start: split,
            ends: Vec::new(),
        }
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

    fn star(&mut self, lazy: bool, ast: &Ast) -> Frag {
        // L1: split L2, L3
        // L2: codes for e
        // jmp L1
        // L3:
        //
        // lazy => split L3, L2
        let l1 = self.push_inst(Inst::Split(vec![]));
        let frag = self.expr(ast);
        let jmp = self.push_inst(Inst::Jmp(l1));
        let l3 = self.next_pos();
        if let Inst::Split(split) = &mut self.insts[l1] {
            *split = if lazy {
                vec![l3, frag.start]
            } else {
                vec![frag.start, l3]
            };
        }
        Frag {
            start: l1,
            ends: frag.ends,
        }
    }

    fn question(&mut self, lazy: bool, ast: &Ast) -> Frag {
        // split L1, L2
        // L1: codes for e
        // L2:
        //
        // lazy => split L2, L1

        let pos = self.push_inst(Inst::Split(vec![]));
        let frag = self.expr(ast);
        let l2 = self.next_pos();
        if let Inst::Split(split) = &mut self.insts[pos] {
            *split = if lazy {
                vec![l2, frag.start]
            } else {
                vec![frag.start, l2]
            };
        }
        Frag {
            start: pos,
            ends: frag.ends,
        }
    }

    fn plus(&mut self, lazy: bool, ast: &Ast) -> Frag {
        // L1: codes for e
        // split L1, L3
        // L3:
        //
        // lazy => split L3, L1
        let frag = self.expr(ast);
        let l3 = self.next_pos() + 1;
        if lazy {
            self.push_inst(Inst::Split(vec![l3, frag.start]));
        } else {
            self.push_inst(Inst::Split(vec![frag.start, l3]));
        }
        frag
    }

    fn group(&mut self, ast: &Ast) -> Frag {
        let group = self.group * 2;
        self.group += 1;

        let save = self.push_inst(Inst::Save(group));
        let mut frag = self.expr(ast);
        self.push_inst(Inst::Save(group + 1));

        frag.start = save;
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
        let regex = ".*?a+b?c*d";
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
        let regex = "a|(b|c)";
        let ast = Parser::parse(regex);
        let program = Compiler::compile(&ast);
        println!("-------- Begin program '{regex}' ---------");
        for (i, inst) in program.iter().enumerate() {
            println!("{i:02}: {inst:?}");
        }
        println!("-------- end program ---------");
    }
}
