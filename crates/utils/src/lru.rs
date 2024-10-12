use std::collections::{HashMap, LinkedList};

#[derive(Debug)]
pub struct LRU<V: Eq> {
    cap: usize,
    list: LinkedList<V>,
}

impl<V: Eq> LRU<V> {
    pub fn new(cap: usize) -> LRU<V> {
        LRU {
            cap,
            list: LinkedList::new(),
        }
    }

    pub fn lookup(&self, value: &V) -> Option<usize> {
        for (i, val) in self.list.iter().enumerate() {
            if val == value {
                return Some(i);
            }
        }

        None
    }

    pub fn insert(&mut self, value: V) {
        let exists = self.list.iter().position(|item| item == &value);

        if let Some(pos) = exists {
            let mut after = self.list.split_off(pos);
            let item = after.pop_front().unwrap();
            self.list.push_front(item);
            self.list.append(&mut after);
            return;
        }

        while self.list.len() >= self.cap {
            self.list.pop_back();
        }

        self.list.push_front(value);
    }
}

impl<V: Eq + std::hash::Hash + Clone> LRU<V> {
    pub fn to_map(&self) -> HashMap<V, usize> {
        let mut map = HashMap::default();
        for (i, val) in self.list.iter().enumerate() {
            map.insert(val.clone(), i);
        }

        map
    }
}
