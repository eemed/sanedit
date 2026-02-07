use std::mem::MaybeUninit;

#[derive(Debug)]
pub struct LinkedArray<T, const N: usize> {
    nodes: [Entry<T>; N],
    head: Option<usize>,
    tail: Option<usize>,
    free: Option<usize>,
}

impl<T, const N: usize> LinkedArray<T, N> {
    pub fn new() -> LinkedArray<T, N> {
        let mut nodes = [(); N].map(|_| Entry::default());
        for (i, entry) in nodes.iter_mut().enumerate() {
            if i != 0 {
                entry.prev = Some(i - 1);
            }

            if i + 1 < N {
                entry.next = Some(i + 1);
            }
        }

        LinkedArray {
            nodes,
            head: None,
            tail: None,
            free: Some(0),
        }
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn is_full(&self) -> bool {
        self.free.is_none()
    }

    fn free_slot(&mut self) -> Option<usize> {
        let free = self.free?;
        self.free = self.nodes[free].next;
        Some(free)
    }

    // Push an element to the front, returns the index in backing array
    // returns none if the array is full
    pub fn push_front(&mut self, val: T) -> Option<usize> {
        let slot = self.free_slot()?;
        let node = &mut self.nodes[slot];

        node.val.write(val);
        node.next = self.head;
        node.prev = None;

        if let Some(head) = self.head {
            self.nodes[head].prev = Some(slot);
        }

        self.head = Some(slot);

        if self.tail.is_none() {
            self.tail = Some(slot);
        }

        Some(slot)
    }

    // Move an element to front
    pub fn move_to_front(&mut self, index: usize) {
        if let Some(next) = self.nodes[index].next {
            self.nodes[next].prev = self.nodes[index].prev;
        }

        if let Some(prev) = self.nodes[index].prev {
            self.nodes[prev].next = self.nodes[index].next;
        }

        let node = &mut self.nodes[index];
        node.next = self.head;
        node.prev = None;

        if let Some(head) = self.head {
            self.nodes[head].prev = Some(index);
        }

        self.head = Some(index);

        if self.tail.is_none() {
            self.tail = Some(index);
        }
    }

    pub fn pop_last(&mut self) -> Option<T> {
        let tail = self.tail?;
        self.tail = self.nodes[tail].prev;
        self.nodes[tail].next = self.free;
        self.nodes[tail].prev = None;
        self.free = Some(tail);
        match self.tail {
            Some(n) => {
                let ntail = &mut self.nodes[n];
                ntail.next = None;
            }
            None => self.head = None,
        }

        let val = std::mem::replace(&mut self.nodes[tail].val, MaybeUninit::uninit());
        // SAFETY: initialized as this item was found in the used list
        Some(unsafe { val.assume_init() })
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            nodes: &self.nodes,
            n: self.head,
        }
    }
}

impl<T: PartialEq, const N: usize> LinkedArray<T, N> {
    pub fn contains(&self, item: &T) -> Option<usize> {
        for (i, val) in self.iter() {
            if val == item {
                return Some(i);
            }
        }

        None
    }
}

impl<T, const N: usize> Default for LinkedArray<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct Entry<T> {
    val: MaybeUninit<T>,
    next: Option<usize>,
    prev: Option<usize>,
}

impl<T> Default for Entry<T> {
    fn default() -> Self {
        Self {
            val: MaybeUninit::uninit(),
            next: Default::default(),
            prev: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a, T> {
    nodes: &'a [Entry<T>],
    n: Option<usize>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.n?;
        let node = &self.nodes[n];
        // Iteration starts from head, so ok
        let value = unsafe { node.val.assume_init_ref() };
        self.n = node.next;
        Some((n, value))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn insert() {
        let mut list = LinkedArray::<usize, 2>::new();
        list.push_front(0);
        list.push_front(1);

        assert_eq!(list.free, None);
        assert_eq!(list.tail, Some(0));
        assert_eq!(list.head, Some(1));

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some((1, &1)));
        assert_eq!(iter.next(), Some((0, &0)));
    }

    #[test]
    fn remove() {
        let mut list = LinkedArray::<usize, 2>::new();
        list.push_front(0);
        list.push_front(1);

        list.pop_last();

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some((1, &1)));
        assert_eq!(iter.next(), None);

        assert_eq!(list.free, Some(0));
        assert_eq!(list.tail, Some(1));
        assert_eq!(list.head, Some(1));

        list.push_front(0);

        let mut iter = list.iter();
        assert_eq!(iter.next(), Some((0, &0)));
        assert_eq!(iter.next(), Some((1, &1)));
    }
}
