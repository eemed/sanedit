use std::mem::MaybeUninit;

///
/// Two ways to use this:
///
/// 1. Normal ring buffer using push_back and take
///     - Will read each item only once
///
/// 2. Storage of N elements and needing non-owning references to them
///     - Use ring buffer as storage
///     - Just overwrite all entries never
///     - Use references to go back and forth
///
#[derive(Debug)]
pub struct RingBuffer<T> {
    items: Box<[MaybeUninit<T>]>,
    write: usize,
    read: usize,
}

impl<T> RingBuffer<T> {
    pub fn with_capacity(cap: usize) -> RingBuffer<T> {
        assert!(
            cap.count_ones() == 1,
            "Ring buffer must have capacity of a power of 2!"
        );

        let mut items = Vec::with_capacity(cap);
        for _ in 0..cap {
            items.push(MaybeUninit::uninit());
        }

        RingBuffer {
            items: items.into_boxed_slice(),
            write: 0,
            read: 0,
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
            self.push(elem);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.read == self.write
    }

    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    pub fn len(&self) -> usize {
        if self.read <= self.write {
            self.write - self.read
        } else {
            self.write + (usize::MAX - self.read + 1)
        }
    }

    fn array_pos(&self, n: usize) -> usize {
        n & (self.capacity() - 1)
    }

    /// Push an element, overwrites last entry if needed
    pub fn push_overwrite(&mut self, item: T) {
        // If full just overwrite
        if self.is_full() {
            self.read = self.read.wrapping_add(1);
        }

        let pos = self.array_pos(self.write);
        self.items[pos].write(item);
        self.write = self.write.wrapping_add(1);
    }

    /// Push an element, returns false if buffer is full
    pub fn push(&mut self, item: T) -> bool {
        if self.is_full() {
            return false;
        }

        let pos = self.array_pos(self.write);
        self.items[pos].write(item);
        self.write = self.write.wrapping_add(1);

        true
    }

    /// Take oldest element in ring buffer and advance read pointer
    pub fn take(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let item = unsafe {
            std::mem::replace(&mut self.items[self.read], MaybeUninit::uninit()).assume_init()
        };
        self.read = self.read.wrapping_add(1);
        Some(item)
    }

    /// Try to read a reference if it has already been overwritten returns None
    pub fn read_reference(&self, reference: &Ref) -> Option<&T> {
        if !self.is_valid_reference(reference) {
            return None;
        }

        let (_, item) = self.read(reference.read)?;
        Some(item)
    }

    /// Look at last written element
    pub fn last(&self) -> Option<(Ref, &T)> {
        if self.is_empty() {
            return None;
        }

        let last_write = self.write.wrapping_sub(1);
        self.read(last_write)
    }

    /// Return whether a reference still points to the same entry
    pub fn is_valid_reference(&self, reference: &Ref) -> bool {
        self.is_valid_read(reference.read)
    }

    fn is_valid_read(&self, read: usize) -> bool {
        if self.read <= self.write {
            self.read <= read && read < self.write
        } else {
            // Probably will never happen but:
            // Write has wrapped, Read has not
            // check if reference was before write or after read
            self.read <= read || read < self.write
        }
    }

    fn read(&self, read: usize) -> Option<(Ref, &T)> {
        let pos = self.array_pos(read);
        let item = unsafe { self.items[pos].assume_init_ref() };
        let item_ref = Ref { read: pos };
        Some((item_ref, item))
    }

    /// Get the previous element from reference
    /// Does not affect read pointer
    pub fn previous(&self, reference: &Ref) -> Option<(Ref, &T)> {
        let previous = reference.read.wrapping_sub(1);

        if !self.is_valid_read(previous) {
            return None;
        }

        self.read(previous)
    }

    /// Get the next element from reference
    /// Does not affect read pointer
    pub fn next(&self, reference: &Ref) -> Option<(Ref, &T)> {
        let next = reference.read.wrapping_add(1);

        if !self.is_valid_read(next) {
            return None;
        }

        self.read(next)
    }
}

/// Keeps a reference to a specific item.
/// Reference will disappear if the element is overwritten in ring buffer
///
/// NOTE: If ring buffer happens to overflow usize and somehow end up in the
/// same position when reference was created it would return a different
/// element. But it would have to wrap around usize before that happens.
///
#[derive(Clone, Debug, PartialEq)]
pub struct Ref {
    read: usize,
}

impl Ref {
    pub fn position(&self) -> usize {
        self.read
    }
}
