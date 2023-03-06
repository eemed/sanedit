pub(crate) type InstIndex = usize;

/// Instructions executed by vms
#[derive(Debug)]
pub(crate) enum Inst {
    Match,
    Byte(u8),
    Jmp(InstIndex),
    Split(Vec<InstIndex>),
}
