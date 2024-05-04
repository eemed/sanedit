use super::set::Set;

pub(crate) type Addr = usize;

#[derive(Debug)]
pub(crate) enum Operation {
    Jump(Addr),
    Byte(u8),
    Call(Addr),
    Commit(Addr),
    Choice(Addr),
    Any(usize),
    UTF8Range(char, char),
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
