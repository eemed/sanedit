use sanedit_utils::bitset::Bitset256;

use crate::CaptureID;

pub(crate) type Addr = usize;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Operation {
    Jump(Addr),
    Byte(u8),
    Call(Addr),
    Commit(Addr),
    Choice(Addr),
    Any(u64),
    UTF8Range(char, char),
    Set(Bitset256),
    Return,
    Fail,
    End,
    PartialCommit(Addr),
    FailTwice,
    Span(Bitset256),
    BackCommit(Addr),
    TestByte(u8, Addr),
    TestSet(Bitset256, Addr),
    CaptureBegin(CaptureID),
    CaptureEnd,
    CaptureLate(CaptureID, u64),
    Backreference(CaptureID),
}
