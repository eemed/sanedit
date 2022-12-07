use super::{chunks::Chunks, Bytes, CursorIterator, PieceTree};

#[derive(Debug)]
pub struct PieceTreeSlice<'a> {
    start: usize,
    end: usize,
    pt: &'a PieceTree,
}

impl<'a> PieceTreeSlice<'a> {
    pub(crate) fn new(pt: &'a PieceTree, start: usize, end: usize) -> PieceTreeSlice {
        PieceTreeSlice { start, end, pt }
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn bytes(&self) -> Bytes {
        self.bytes_at(self.start)
    }

    #[inline]
    pub fn bytes_at(&self, pos: usize) -> Bytes {
        debug_assert!(
            self.start + pos <= self.pt.len,
            "bytes_at: Attempting to index {} over buffer len {}",
            self.start + pos,
            self.pt.len
        );
        Bytes::new(self.pt, self.start + pos)
    }

    #[inline]
    pub fn chunks(&self) -> Chunks {
        self.chunks_at(self.start)
    }

    #[inline]
    pub fn chunks_at(&self, pos: usize) -> Chunks {
        debug_assert!(
            self.start + pos <= self.pt.len,
            "chunks_at: Attempting to index {} over buffer len {}",
            self.start + pos,
            self.pt.len
        );
        Chunks::new(self.pt, self.start + pos)
    }
}

impl<'a, B: AsRef<[u8]>> PartialEq<B> for PieceTreeSlice<'a> {
    fn eq(&self, other: &B) -> bool {
        let mut total = 0;
        let other = other.as_ref();
        let mut chunks = self.chunks();
        let mut chunk = chunks.get();

        while let Some(chk) = chunk {
            let chk_pos = chunks.pos();
            let chk_bytes = &chk.as_ref()[self.start.saturating_sub(chk_pos)..];
            let chk_len = chk_bytes.len();

            if chk_bytes != &other[..chk_len] {
                return false;
            }

            total += chk_len;

            chunk = chunks.next();
        }

        total == other.len()
    }
}
