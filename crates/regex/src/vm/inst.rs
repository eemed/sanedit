pub(crate) type InstIndex = usize;
pub(crate) type InstOffset = isize;

/// Instructions executed by vms
#[derive(Debug, Clone)]
pub(crate) enum Inst {
    Match,
    Byte(u8),
    ByteRange(u8, u8),
    Jmp(InstOffset),
    Split(InstOffset, InstOffset),
    Save(usize),
}
