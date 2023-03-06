mod compiler;
mod inst;
mod program;
mod thread;

pub(crate) use compiler::Compiler;
pub(crate) use program::Program;

use std::mem;

use crate::cursor::Cursor;

use self::inst::Inst;
use self::thread::ThreadSet;

pub(crate) struct VM;

impl VM {
    /// Run a program on thompsons vm
    pub(crate) fn thompson(program: &Program, input: &mut impl Cursor) {
        let len = program.len();
        // sp
        let mut current = ThreadSet::with_capacity(len);
        let mut new = ThreadSet::with_capacity(len);

        current.add_thread(0);

        while let Some(byte) = input.next() {
            let mut i = 0;
            while i < current.len() {
                use Inst::*;

                let pc = current[i];
                match &program[pc] {
                    Match => {
                        return;
                    }
                    Byte(inst_byte) => {
                        if byte != *inst_byte {
                            break;
                        }
                        new.add_thread(pc + 1)
                    }
                    Jmp(x) => {
                        current.add_thread(*x);
                    }
                    Split(splits) => {
                        for split in splits {
                            current.add_thread(*split);
                        }
                    }
                }

                i += 1;
            }
            mem::swap(&mut current, &mut new);
            new.clear();
        }
    }
}

// int thompsonvm(Inst *prog, char *input)
// {
//     int len;
//     ThreadList *clist, *nlist;
//     Inst *pc;
//     char *sp;

//     len = proglen(prog);  /* # of instructions */
//     clist = threadlist(len);
//     nlist = threadlist(len);

//     addthread(clist, thread(prog));
//     for(sp=input; *sp; sp++){
//         for(i=0; i<clist.n; i++){
//             pc = clist.t[i].pc;
//             switch(pc->opcode){
//                 case Char:
//                     if(*sp != pc->c)
//                         break;
//                     addthread(nlist, thread(pc+1));
//                     break;
//                 case Match:
//                     return 1;
//                 case Jmp:
//                     addthread(clist, thread(pc->x));
//                     break;
//                 case Split:
//                     addthread(clist, thread(pc->x));
//                     addthread(clist, thread(pc->y));
//                     break;
//             }
//         }
//         swap(clist, nlist);
//         clear(nlist);
//     }
// }
