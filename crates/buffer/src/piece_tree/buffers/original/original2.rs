use std::collections::BTreeMap;

enum Block {
    Mmap {},
    File {},
    Memory {},
}

struct Blocks {
    map: BTreeMap<usize, Block>,
}

struct OriginalBuffer {
    blocks: Blocks,
}
