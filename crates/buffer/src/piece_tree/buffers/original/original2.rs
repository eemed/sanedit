use std::{collections::BTreeMap, io};

enum Block {
    // File { },
    Mmap { map: memmap::Mmap },
    Memory { bytes: Vec<u8> },
}

type Blocks = BTreeMap<usize, Block>;

struct OriginalBuffer {
    blocks: Blocks,
}

impl OriginalBuffer {
    #[inline]
    pub fn new() -> OriginalBuffer {
        OriginalBuffer {
            blocks: Blocks::new(),
        }
    }

    #[inline]
    pub fn from_reader<T: io::Read>(mut reader: T) -> io::Result<OriginalBuffer> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let block = Block::Memory { bytes };
        let mut blocks = Blocks::new();
        blocks.insert(0, block);
        Ok(OriginalBuffer { blocks })
    }

    /// Returns the length of the original buffer
    pub fn len(&self) -> usize {
        0
    }

    /// Returns the length of the backing file if file backed
    pub fn file_len(&self) -> usize {
        0
    }
}
