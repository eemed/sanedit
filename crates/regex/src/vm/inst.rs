use std::ops::Range;

pub(crate) type InstIndex = usize;

/// Instructions executed by vms
#[derive(Debug)]
pub(crate) enum Inst {
    Match,
    Byte(u8),
    ByteRange(Range<u8>),
    Jmp(InstIndex),
    Split(Vec<InstIndex>),
    Save(usize),
}
