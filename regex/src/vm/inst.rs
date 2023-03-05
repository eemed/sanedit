pub type InstPtr = usize;

/// Instructions executed by vms
pub enum Inst {
    Match,
    Char(char),
    Jmp(InstPtr),
    Split(InstPtr, InstPtr),
}
