pub(crate) type InstPtr = usize;

/// Instructions executed by vms
pub(crate) enum Inst {
    Match,
    Char(char),
    Jmp(InstPtr),
    Split(InstPtr, InstPtr),
}
