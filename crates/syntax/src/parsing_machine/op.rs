use crate::CaptureID;

use super::set::Set;

pub(crate) type Addr = usize;

#[allow(dead_code)]
#[derive(Debug)]
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
    EndFail,
    PartialCommit(Addr),
    FailTwice,
    Span(Set),
    BackCommit(Addr),
    TestChar(u8, Addr),
    TestCharNoChoice(u8, Addr),
    TestSet(Set, Addr),
    TestSetNoChoice(Set, Addr),
    TestAny(usize, Addr),
    CaptureBegin(CaptureID),
    CaptureEnd,
    Checkpoint,
}
