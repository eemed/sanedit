pub(crate) type InstPtr = usize;

/// Instructions executed by vms
pub(crate) enum Inst {
    Match,
    Byte(u8),
    Jmp(InstPtr),
    Split(InstPtr, InstPtr),
}
