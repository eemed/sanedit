use std::collections::LinkedList;

pub struct LRU<T> {
    cap: usize,
    list: LinkedList<T>,
}

impl<T> LRU<T> {
    pub fn new(cap: usize) -> LRU<T> {
        todo!();
    }
}
