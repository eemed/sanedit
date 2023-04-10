use std::ops::Range;

pub(crate) type InstIndex = usize;
pub(crate) type InstOffset = isize;

/// Instructions executed by vms
#[derive(Debug)]
pub(crate) enum Inst {
    Match,
    Byte(u8),
    ByteRange(Range<u8>),
    Jmp(InstOffset),
    Split(InstOffset, InstOffset),
    Save(usize),
}
