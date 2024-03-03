use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct MemoTable {
    table: HashMap<MemoKey, Match>,
}

impl MemoTable {
    pub fn new() -> MemoTable {
        MemoTable {
            table: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct MemoKey {
    /// Rule index
    pub rule: usize,
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
