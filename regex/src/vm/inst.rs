// enum {    /* Inst.opcode */
//         Char,
//         Match,
//         Jmp,
//         Split
// };

// struct Inst {
//    int opcode;
//    int c;
//    Inst *x;
//    Inst *y;
// };

/// Instructions executed by vms
pub enum Inst {
    Match,
    Char(char),
    Jmp(InstPtr),
    Split(InstPtr, InstPtr),
}

pub type InstPtr = usize;
