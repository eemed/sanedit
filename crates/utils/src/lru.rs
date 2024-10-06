use std::collections::LinkedList;

pub struct LRU<T> {
    cap: usize,
    list: LinkedList<T>,
}

impl<T> LRU<T> {
    fn new(cap: usize) -> LRU<T> {
        todo!()
    }
}
