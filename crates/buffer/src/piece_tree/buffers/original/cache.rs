use std::{collections::VecDeque, sync::Arc};

use super::slice::OriginalBufferSlice;

#[derive(Debug)]
pub(crate) struct Cache {
    /// Buffer data. (buf_offset, data) tuples
    pub(super) cache: VecDeque<(usize, Arc<[u8]>)>,
}

impl Cache {
    const CACHE_SIZE: usize = 10;

    pub fn new() -> Cache {
        Cache {
            cache: VecDeque::new(),
        }
    }

    pub fn get(&self, start: usize, end: usize) -> Option<OriginalBufferSlice> {
        for (off, ptr) in &self.cache {
            if *off <= start && end <= off + ptr.len() {
                let s = start - off;
                let e = s + end - start;
                return Some(OriginalBufferSlice {
                    ptr: ptr.clone(),
                    offset: s,
                    len: e - s,
                });
            }
        }

        None
    }

    pub fn push(&mut self, off: usize, ptr: Arc<[u8]>) -> OriginalBufferSlice {
        while self.cache.len() >= Self::CACHE_SIZE {
            self.cache.pop_front();
        }

        self.cache.push_back((off, ptr.clone()));

        OriginalBufferSlice {
            offset: 0,
            len: ptr.len(),
            ptr,
        }
    }
}
