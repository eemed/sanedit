use std::{collections::BTreeMap, fs::File, io, ops::Range};

use crate::piece_tree::{buffers::ByteSlice, tree::piece::Piece};

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

    #[inline]
    pub fn from_file(file: File) -> OriginalBuffer {
        todo!()
    }

    #[inline]
    pub fn mmap(file: File) -> io::Result<OriginalBuffer> {
        let mmap = unsafe { memmap::Mmap::map(&file)? };
        let block = Block::Mmap { map: mmap };
        let mut blocks = Blocks::new();
        blocks.insert(0, block);
        Ok(OriginalBuffer { blocks })
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> io::Result<ByteSlice<'_>> {
        todo!()
    }

    /// Returns the length of the original buffer
    pub fn len(&self) -> usize {
        0
    }

    /// Returns the length of the backing file if file backed
    pub fn file_len(&self) -> usize {
        0
    }

    /// Wether the content the piece is referring to is written in the backing
    /// file at position pos
    pub fn is_in_file(pos: usize, piece: &Piece) -> bool {
        false
    }
}
