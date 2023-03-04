mod inst;
mod program;
mod thread;

use std::mem;

use crate::{vm::inst::Inst, cursor::CharCursor};

use self::{program::Program, thread::ThreadSet, inst::InstPtr};

// TODO input
pub fn thompson_vm(program: Program, input: impl CharCursor) {
    let len = program.len();
    let mut pc: InstPtr = 0;
    // sp
    let mut current = ThreadSet::with_capacity(len);
    let mut new = ThreadSet::with_capacity(len);

    while let Some(ch) = input.next() {
        for pc in current.iter() {
            use Inst::*;
            match &program[*pc] {
                Match => {
                    return;
                }
                Char(inst_ch) => {
                    // if(*sp != pc->c)
                    //     break;
                    new.add_thread(*pc + 1)
                },
                Jmp(x) => {
                    current.add_thread(x);
                },
                Split(x, y) => {
                    current.add_thread(x);
                    current.add_thread(y);
                }
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
