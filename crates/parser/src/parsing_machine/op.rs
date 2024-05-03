use super::set::Set;

pub(crate) type Addr = usize;

pub(crate) enum Operation {
    Jump(Addr),
    Byte(u8),
    Call(Addr),
    Commit(Addr),
    Choice(Addr),
    Any(usize),
    UnicodeRange(char, char),
    Set(Set),
    Return,
    Fail,
    End,
    EndFail,
    PartialCommit(Addr),
    FailTwice,
    Span(Set),
    TestSetNoChoice(Set),
    BackCommit(Addr),
}
