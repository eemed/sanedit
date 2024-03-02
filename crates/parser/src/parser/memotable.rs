use std::collections::HashMap;

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
    rule: usize,
    /// Input start position
    start: usize,
}

impl MemoKey {}

pub(crate) struct Match {
    /// Length of the match
    len: usize,
}

impl Match {}
