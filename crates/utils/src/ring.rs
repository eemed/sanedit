use std::mem::MaybeUninit;

/// Ring buffer that never deletes, only overwrites entries from the backing buffer
///
/// Two ways to use this:
///
/// 1. Normal ring buffer using push_back and take
///     - Will read each item only once
///
/// 2. Storage of N elements and needing non-owning references to them
///     - Use ring buffer as storage
///     - iter will iterate all buffer contents
///         - Iterator yields references that can be kept even if more items are
///         pushed
///         - References can be then retrieved if element was not overwritten
///
#[derive(Debug)]
pub struct RingBuffer<T> {
    items: Box<[MaybeUninit<T>]>,
    write: usize,
    read: usize,

    /// How many times the whole buffer has been overwritten
    overwritten: usize,
}

impl<T> RingBuffer<T> {
    pub fn with_capacity(cap: usize) -> RingBuffer<T> {
        assert!(cap != 0, "Ring buffer with capacity 0!");

        let mut items = Vec::with_capacity(cap);
        for _ in 0..cap {
            items.push(MaybeUninit::uninit());
        }

        RingBuffer {
            items: items.into_boxed_slice(),
            write: 0,
            read: 0,
            overwritten: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.items.len()
    }

    pub fn extend<I>(&mut self, i: I)
    where
        I: IntoIterator<Item = T>,
    {
        for elem in i.into_iter() {
            self.push_back(elem);
        }
    }

    /// Push an element, overwrites last entry if needed
    pub fn push_back_overwrite(&mut self, item: T) {
        let next = (self.write + 1) % self.items.len();
        if next == self.read {
            self.read = (self.read + 1) % self.items.len();
        }

        self.items[self.write].write(item);
        self.write = next;

        if self.write == 0 {
            self.overwritten += 1;
        }
    }

    /// Push an element, returns false if buffer is full
    pub fn push_back(&mut self, item: T) -> bool {
        let next = (self.write + 1) % self.items.len();
        if next == self.read {
            return false;
        }

        self.items[self.write].write(item);
        self.write = next;

        if self.write == 0 {
            self.overwritten += 1;
        }

        true
    }

    pub fn can_read(&self) -> bool {
        return self.read != self.write;
    }

    /// Take oldest element in ring buffer and advance read pointer
    pub fn read(&mut self) -> Option<&T> {
        if self.read == self.write {
            return None;
        }

        let item = unsafe { self.items[self.read].assume_init_ref() };
        self.read = (self.read + 1) % self.items.len();
        Some(item)
    }

    /// Total amount of elements stored. Note that elements are never removed
    pub fn total_stored(&self) -> usize {
        if self.overwritten != 0 {
            return self.items.len();
        }

        self.write
    }

    /// Look at last written element
    pub fn last(&self) -> Option<(Ref, &T)> {
        let index = self.last_index()?;
        self.get(index)
    }

    /// Last written element index
    pub fn last_index(&self) -> Option<usize> {
        if self.write == 0 && self.overwritten == 0 {
            return None;
        }

        if self.write == 0 {
            Some(self.items.len() - 1)
        } else {
            Some(self.write - 1)
        }
    }

    /// Get the previous element from reference
    /// Does not affect read pointer
    pub fn previous(&self, reference: &Ref) -> Option<(Ref, &T)> {
        if reference.position == 0 && reference.overwritten == 0 {
            return None;
        }

        if reference.position == 0 {
            self.get(self.items.len() - 1)
        } else {
            self.get(reference.position - 1)
        }
    }

    /// Get the next element from reference
    /// Does not affect read pointer
    pub fn next(&self, reference: &Ref) -> Option<(Ref, &T)> {
        todo!()
    }

    /// Get element at index
    /// Does not affect read pointer
    pub fn get(&self, index: usize) -> Option<(Ref, &T)> {
        if self.overwritten == 0 && index >= self.write {
            return None;
        }

        let item = unsafe { self.items[index].assume_init_ref() };
        let item_ref = Ref {
            position: index,
            overwritten: self.overwritten,
        };
        Some((item_ref, item))
    }

    // /// Get element at reference
    // /// Does not affect read pointer
    // pub fn get_ref(&self, reference: &Ref) -> Option<&T> {
    //     let current_loop =
    //         reference.overwritten == self.overwritten && reference.position < self.write;
    //     let prev_loop = self.overwritten != 0
    //         && reference.overwritten == self.overwritten - 1
    //         && reference.position >= self.write;
    //     if current_loop || prev_loop {
    //         let item = unsafe { self.items[reference.position].assume_init_ref() };
    //         return Some(item);
    //     }

    //     None
    // }
}

/// Keeps a reference to a specific item.
/// Reference will disappear if the element is overwritten in ring buffer
#[derive(Clone, Debug, PartialEq)]
pub struct Ref {
    position: usize,
    overwritten: usize,
}

impl Ref {
    pub fn position(&self) -> usize {
        self.position
    }
}
