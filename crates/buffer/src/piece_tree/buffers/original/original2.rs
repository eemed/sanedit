use std::{collections::BTreeMap, fs::File, io, ops::Range};

use crate::piece_tree::{buffers::ByteSlice, tree::piece::Piece};

enum Block {
    // File { },
    Mmap { map: memmap::Mmap },
    Memory { bytes: Vec<u8> },
}

impl Block {
    pub fn slice(&self, range: Range<usize>) -> &[u8] {
        match self {
            Block::Mmap { map } => &map[range],
            Block::Memory { bytes } => &bytes[range],
        }
    }
}

type Blocks = BTreeMap<usize, Block>;

enum OriginalBuffer {
    FileBacked { blocks: Blocks, len: usize },
    Memory { bytes: Vec<u8> },
}

impl OriginalBuffer {
    #[inline]
    pub fn new() -> OriginalBuffer {
        OriginalBuffer::Memory { bytes: Vec::new() }
    }

    #[inline]
    pub fn from_reader<T: io::Read>(mut reader: T) -> io::Result<OriginalBuffer> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(OriginalBuffer::Memory { bytes })
    }

    #[inline]
    pub fn from_file(file: File) -> OriginalBuffer {
        todo!()
    }

    #[inline]
    pub fn mmap(file: File) -> io::Result<OriginalBuffer> {
        let mmap = unsafe { memmap::Mmap::map(&file)? };
        let len = mmap.len();
        let block = Block::Mmap { map: mmap };
        let mut blocks = Blocks::new();
        blocks.insert(0, block);
        Ok(OriginalBuffer::FileBacked { blocks, len })
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> io::Result<ByteSlice<'_>> {
        use OriginalBuffer::*;
        match self {
            Memory { bytes } => Ok(bytes[range].into()),
            FileBacked { blocks, len } => {
                let mut iter = blocks.range(range).peekable();
                let (pos, block) = iter.next().unwrap();
                let is_multi_block = iter.peek().is_some();

                if is_multi_block {
                    todo!()
                } else {
                    Ok(block.slice(range).into())
                }
            }
        }
    }

    /// Returns the length of the original buffer
    pub fn len(&self) -> usize {
        use OriginalBuffer::*;
        match self {
            FileBacked { blocks, len } => *len,
            Memory { bytes } => bytes.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_file_backed(&self) -> bool {
        use OriginalBuffer::*;
        match self {
            FileBacked { .. } => true,
            _ => false,
        }
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
