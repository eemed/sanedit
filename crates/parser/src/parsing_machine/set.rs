use std::ops::Index;

use bitvec::{bitarr, prelude::*};

pub(crate) struct Set {
    inner: BitArray<[u32; 8]>,
}

impl Set {
    pub fn new() -> Set {
        Set {
            inner: bitarr![u32, Lsb0; 0; 256],
        }
    }

    pub fn add(&mut self, n: u8) {
        self.inner.set(n as usize, true);
    }
}

impl Index<u8> for Set {
    type Output = bool;

    fn index(&self, index: u8) -> &Self::Output {
        &self.inner[index as usize]
    }
}

impl Index<usize> for Set {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}
