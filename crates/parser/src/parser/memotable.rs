use std::collections::HashMap;

pub(crate) type MemoTable = HashMap<MemoKey, Match>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct MemoKey {
    pub clause: usize,
    /// Input start position
    pub start: usize,
}

impl MemoKey {}

#[derive(Debug)]
pub(crate) struct Match {
    pub key: MemoKey,

    /// Length of the match
    pub len: usize,
}

impl Match {}
