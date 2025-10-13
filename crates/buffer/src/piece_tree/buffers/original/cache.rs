use std::sync::Arc;

use super::slice::OriginalBufferSlice;

#[derive(Debug)]
pub(crate) struct BufferPart {
    off: u64,
    ptr: Arc<[u8]>,
}

#[derive(Debug)]
pub(crate) struct Cache {
    n: usize,
    parts: [BufferPart; 8],
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            n: 0,
            parts: std::array::from_fn(|_| BufferPart {
                off: 0,
                ptr: Arc::new([]),
            }),
        }
    }

    pub fn get(&self, start: u64, end: u64) -> Option<OriginalBufferSlice> {
        for part in &self.parts {
            let BufferPart { off, ptr } = part;
            if *off <= start && end <= off + ptr.len() as u64 {
                let s = start - off;
                let e = s + end - start;
                return Some(OriginalBufferSlice {
                    ptr: ptr.clone(),
                    offset: s as usize,
                    len: (e - s) as usize,
                });
            }
        }

        None
    }

    pub fn push(&mut self, off: u64, ptr: Arc<[u8]>) -> OriginalBufferSlice {
        let part = BufferPart {
            off,
            ptr: ptr.clone(),
        };

        self.parts[self.n] = part;
        self.n = (self.n + 1) % self.parts.len();

        OriginalBufferSlice {
            offset: 0,
            len: ptr.len(),
            ptr,
        }
    }
}
