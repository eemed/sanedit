use std::{collections::LinkedList, sync::Arc};

use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct LRU<V: Eq> {
    cap: usize,
    list: LinkedList<V>,

    /// Keep hashmap of value positions here for easy access
    map: Arc<FxHashMap<V, usize>>,
    /// Whether a new value has been inserted after the map has been constructed
    updated: bool,
}

impl<V: Eq> LRU<V> {
    pub fn new(cap: usize) -> LRU<V> {
        LRU {
            cap,
            list: LinkedList::new(),

            map: Arc::new(FxHashMap::default()),
            updated: false,
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
        self.updated = true;
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
    pub fn to_map(&mut self) -> Arc<FxHashMap<V, usize>> {
        if !self.updated {
            return self.map.clone();
        }

        let mut map = FxHashMap::default();
        for (i, val) in self.list.iter().enumerate() {
            map.insert(val.clone(), i);
        }

        self.map = Arc::new(map);
        self.map.clone()
    }
}
