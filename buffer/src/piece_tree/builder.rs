use super::{buffers::OriginalBuffer, PieceTree};

#[derive(Debug)]
pub struct PieceTreeBuilder {
    buf: Vec<u8>,
}

impl PieceTreeBuilder {
    pub fn new() -> PieceTreeBuilder {
        PieceTreeBuilder { buf: Vec::new() }
    }

    pub fn append(&mut self, string: &str) {
        self.buf.extend_from_slice(string.as_bytes());
    }

    pub fn build(self) -> PieceTree {
        let orig_buf = OriginalBuffer::Memory { bytes: self.buf };
        PieceTree::from_original_buffer(orig_buf)
    }
}

impl Default for PieceTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
