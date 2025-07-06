use crate::CaptureID;

use super::set::Set;

pub(crate) type Addr = usize;

#[allow(dead_code)]
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Operation {
    Jump(Addr),
    Byte(u8),
    Call(Addr),
    Commit(Addr),
    Choice(Addr),
    Any(u64),
    UTF8Range(char, char),
    Set(Set),
    Return,
    Fail,
    End,
    PartialCommit(Addr),
    FailTwice,
    Span(Set),
    BackCommit(Addr),
    TestChar(u8, Addr),
    TestSet(Set, Addr),
    CaptureBegin(CaptureID),
    CaptureEnd,
}
