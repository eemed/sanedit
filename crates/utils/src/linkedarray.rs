#[derive(Debug)]
pub struct LinkedArray<T, const N: usize> {
    nodes: [Entry<T>; N],
    head: Option<usize>,
    tail: Option<usize>,
    free: Option<usize>,
    len: usize,
}

#[derive(Debug)]
struct Entry<T> {
    val: Option<T>,
    next: Option<usize>,
    prev: Option<usize>,
}

impl<T, const N: usize> LinkedArray<T, N> {
    pub fn new() -> Self {
        let nodes: [Entry<T>; N] = std::array::from_fn(|i| Entry {
            val: None,
            prev: if i > 0 { Some(i - 1) } else { None },
            next: if i + 1 < N { Some(i + 1) } else { None },
        });

        Self {
            nodes,
            head: None,
            tail: None,
            free: Some(0),
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == N
    }

    fn take_free(&mut self) -> Option<usize> {
        let idx = self.free?;
        self.free = self.nodes[idx].next;
        self.nodes[idx].next = None;
        self.nodes[idx].prev = None;
        Some(idx)
    }

    pub fn push_front(&mut self, val: T) -> Option<usize> {
        let idx = self.take_free()?;

        self.nodes[idx].val = Some(val);
        self.nodes[idx].next = self.head;
        self.nodes[idx].prev = None;

        if let Some(head) = self.head {
            self.nodes[head].prev = Some(idx);
        }

        self.head = Some(idx);

        if self.tail.is_none() {
            self.tail = Some(idx);
        }

        self.len += 1;
        Some(idx)
    }

    pub fn move_to_front(&mut self, index: usize) {
        if index >= N {
            return;
        }

        if self.nodes[index].val.is_none() {
            return; // not in active list
        }

        if Some(index) == self.head {
            return;
        }

        let prev = self.nodes[index].prev;
        let next = self.nodes[index].next;

        if let Some(p) = prev {
            self.nodes[p].next = next;
        }

        if let Some(n) = next {
            self.nodes[n].prev = prev;
        }

        if Some(index) == self.tail {
            self.tail = prev;
        }

        self.nodes[index].prev = None;
        self.nodes[index].next = self.head;

        if let Some(head) = self.head {
            self.nodes[head].prev = Some(index);
        }

        self.head = Some(index);
    }

    pub fn pop_last(&mut self) -> Option<T> {
        let tail = self.tail?;

        let prev = self.nodes[tail].prev;

        if let Some(p) = prev {
            self.nodes[p].next = None;
        } else {
            self.head = None;
        }

        self.tail = prev;
        let val = self.nodes[tail].val.take();

        // return node to free list
        self.nodes[tail].next = self.free;
        self.nodes[tail].prev = None;
        self.free = Some(tail);

        self.len -= 1;
        val
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            nodes: &self.nodes,
            current: self.head,
        }
    }
}

pub struct Iter<'a, T> {
    nodes: &'a [Entry<T>],
    current: Option<usize>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.current?;
        let node = &self.nodes[idx];
        let val = node.val.as_ref()?;
        self.current = node.next;
        Some((idx, val))
    }
}

impl<T, const N: usize> Default for LinkedArray<T, N> {
    fn default() -> Self {
        Self::new()
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
        assert_eq!(iter.next(), None);
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
        assert_eq!(iter.next(), None);
    }
}
