use std::{
    collections::BTreeMap,
    marker::PhantomData,
    ops::{Index, IndexMut},
    sync::atomic::{AtomicU32, Ordering},
};

#[macro_export]
macro_rules! key_type {
    ($vis:vis $name:ident) => {
        #[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
        $vis struct $name(sanedit_utils::idmap::ID);

        impl sanedit_utils::idmap::AsID for $name {
            fn id(&self) -> sanedit_utils::idmap::ID {
                self.0
            }

            fn to_id(id: sanedit_utils::idmap::ID) -> Self {
                $name(id)
            }
        }

        impl From<sanedit_utils::idmap::ID> for $name {
            fn from(id: sanedit_utils::idmap::ID) -> $name {
                $name(id)
            }
        }
    }
}

pub trait AsID {
    fn id(&self) -> ID;
    fn to_id(id: ID) -> Self;
}

pub type ID = u32;

/// Map that stores values and returns their id.
/// TODO?: IDs are not reused, should they?
#[derive(Debug)]
pub struct IdMap<K: AsID, V> {
    map: BTreeMap<ID, V>,
    next_id: AtomicU32,
    _phantom: PhantomData<K>,
}

impl<K: AsID, V> IdMap<K, V> {
    pub fn insert(&mut self, value: V) -> K {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.map.insert(id, value);
        K::to_id(id)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(&key.id())
    }

    pub fn iter(&self) -> Iter<K, V> {
        let iter = self.map.iter();
        Iter {
            iter,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(&key.id())
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(&key.id())
    }
}

impl<K: AsID, V> Index<K> for IdMap<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.map.get(&index.id()).unwrap()
    }
}

impl<K: AsID, V> IndexMut<K> for IdMap<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.map.get_mut(&index.id()).unwrap()
    }
}

pub struct Iter<'a, K: AsID, V> {
    iter: std::collections::btree_map::Iter<'a, ID, V>,
    _phantom: PhantomData<K>,
}

impl<'a, K: AsID, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, val) = self.iter.next()?;
        let id = K::to_id(*id);

        Some((id, val))
    }
}

impl<K: AsID, V> Default for IdMap<K, V> {
    fn default() -> Self {
        Self {
            map: BTreeMap::default(),
            next_id: AtomicU32::new(1),
            _phantom: PhantomData,
        }
    }
}
