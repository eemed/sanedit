use std::fmt;

// TODO convert into a bitset, takes less space
pub(crate) struct Set {
    items: Box<[bool]>,
}

impl Set {
    pub fn new(n: usize) -> Set {
        Set {
            items: vec![false; n].into(),
        }
    }

    pub fn new_all(n: usize) -> Set {
        Set {
            items: vec![true; n].into(),
        }
    }

    pub fn insert(&mut self, n: usize) {
        self.items[n] = true;
    }

    pub fn remove(&mut self, n: usize) {
        self.items[n] = false;
    }

    pub fn contains(&self, n: usize) -> bool {
        self.items[n]
    }

    pub fn to_vec(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(i, b)| **b)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        SetIter { set: self, i: 0 }
    }

    pub fn union(&mut self, other: Set) {
        for o in other.iter() {
            self.insert(o);
        }
    }
}

struct SetIter<'a> {
    set: &'a Set,
    i: usize,
}

impl<'a> Iterator for SetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.i >= self.set.len() {
                return None;
            }

            let i = self.i;
            let b = self.set.contains(i);
            self.i += 1;

            if b {
                return Some(i);
            }
        }
    }
}

impl fmt::Debug for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        write!(f, "Set {{")?;
        for (i, b) in self.items.iter().enumerate() {
            if *b {
                if !first {
                    write!(f, ", ")?;
                }
                first = false;

                write!(f, "{}", i)?;
            }
        }
        write!(f, "}}")
    }
}
