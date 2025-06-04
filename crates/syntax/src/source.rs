use std::{
    cmp::min,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use sanedit_buffer::{utf8::decode_utf8_iter, Bytes};

pub trait ByteSource {
    /// Length of all the bytes in this reader utf8
    fn len(&self) -> u64;

    /// Wether to stop parsing and return an error
    fn stop(&self) -> bool;

    fn get(&mut self, at: u64) -> u8;

    fn matches(&mut self, at: u64, exp: &[u8]) -> bool {
        if at + exp.len() as u64 >= self.len() {
            return false;
        }

        let mut cur = at;
        for e in exp {
            let byte = self.get(cur);
            cur += 1;
            if *e != byte {
                return false;
            }
        }

        true
    }

    fn char_between(&mut self, at: u64, start: char, end: char) -> Option<u64> {
        let max = min(4, self.len() - at);
        let mut bytes = [0u8; 4];
        for i in 0..max {
            bytes[i as usize] = self.get(at + i);
        }
        let (ch, size) = decode_utf8_iter(bytes[..max as usize].iter().copied());
        let ch = ch?;

        if start <= ch && ch <= end {
            Some(size)
        } else {
            None
        }
    }
}

impl<'a> ByteSource for &'a str {
    fn len(&self) -> u64 {
        self.as_bytes().len() as u64
    }

    fn stop(&self) -> bool {
        false
    }

    fn get(&mut self, at: u64) -> u8 {
        self.as_bytes()[at as usize]
    }
}

impl<'a> ByteSource for &'a [u8] {
    fn len(&self) -> u64 {
        <[u8]>::len(self) as u64
    }

    fn stop(&self) -> bool {
        false
    }

    fn get(&mut self, at: u64) -> u8 {
        self[at as usize]
    }
}

impl<'a> ByteSource for Bytes<'a> {
    fn len(&self) -> u64 {
        <Bytes>::len(self)
    }

    fn stop(&self) -> bool {
        false
    }

    fn get(&mut self, at: u64) -> u8 {
        <Bytes>::at(self, at)
    }
}

impl<B: ByteSource> ByteSource for (B, Arc<AtomicBool>) {
    fn len(&self) -> u64 {
        self.0.len()
    }

    fn stop(&self) -> bool {
        self.1.load(Ordering::Acquire)
    }

    fn get(&mut self, at: u64) -> u8 {
        self.0.get(at)
    }
}

impl<const N: usize> ByteSource for &[u8; N] {
    #[inline(always)]
    fn len(&self) -> u64 {
        N as u64
    }

    #[inline(always)]
    fn get(&mut self, i: u64) -> u8 {
        self[i as usize]
    }

    fn stop(&self) -> bool {
        false
    }
}
