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
pub struct RingBuffer<T, const N: usize> {
    items: [MaybeUninit<T>; N],
    write: usize,
    read: usize,
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        assert!(
            N.count_ones() == 1,
            "Ring buffer must have capacity of a power of 2!"
        );
        assert!(
            N <= (usize::MAX >> 1),
            "Ring buffer must have capacity lower than {}!",
            (usize::MAX >> 1)
        );

        RingBuffer {
            items: unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() },
            write: 0,
            read: 0,
        }
    }
}

impl<T, const N: usize> RingBuffer<T, N> {
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
        self.write.wrapping_sub(self.read)
    }

    fn position(&self, n: usize) -> usize {
        n & (self.capacity() - 1)
    }

    /// Push an element, overwrites last entry if needed
    pub fn push_overwrite(&mut self, item: T) {
        // If full just overwrite
        if self.is_full() {
            self.read = self.read.wrapping_add(1);
        }

        let pos = self.position(self.write);
        self.items[pos].write(item);
        self.write = self.write.wrapping_add(1);
    }

    /// Push an element, returns false if buffer is full
    pub fn push(&mut self, item: T) -> bool {
        if self.is_full() {
            return false;
        }

        let pos = self.position(self.write);
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

    fn read_index(&self, read: usize) -> Option<&T> {
        let pos = self.position(read);
        let item = unsafe { self.items[pos].assume_init_ref() };
        Some(item)
    }
}

impl<T, const N: usize> Drop for RingBuffer<T, N> {
    fn drop(&mut self) {
        let mut idx = self.read;
        while idx != self.write {
            // SAFETY: The range from read to write contains initialized elements
            unsafe {
                let ptr = self.items[idx].as_mut_ptr();
                std::ptr::drop_in_place(ptr);
            }
            idx = (idx + 1) % N;
        }
    }
}

/// Sometimes random access to a ring is needed
/// This trait provides a way to reference items in a Weak ref manner.
/// This avoids allocation for each element that would be needed with Rc or Arc.
/// The trait is separated from the main ring functionality for clarity
/// The functions do not affect the ring buffers natural read mechanism.
pub trait RingItemReference {
    /// Item type
    type T;
    /// Reference type
    type R;

    /// Get the next element from reference
    /// Does not affect read pointer
    fn next_of_ref(&self, reference: &Self::R) -> Option<(Self::R, &Self::T)>;

    /// Get the previous element from reference
    /// Does not affect read pointer
    fn previous_of_ref(&self, reference: &Self::R) -> Option<(Self::R, &Self::T)>;

    /// Return whether a reference still points to the same entry
    fn is_valid_reference(&self, reference: &Self::R) -> bool;

    /// Try to read a reference if it has already been overwritten returns None
    fn read_reference(&self, reference: &Self::R) -> Option<&Self::T>;

    /// Read last written element
    fn last(&self) -> Option<(Self::R, &Self::T)>;
}

impl<T, const N: usize> RingItemReference for RingBuffer<T, N> {
    type T = T;
    type R = Ref;

    fn next_of_ref(&self, reference: &Ref) -> Option<(Ref, &Self::T)> {
        let next = reference.read.wrapping_add(1);

        if !self.is_valid_read(next) {
            return None;
        }

        let refe = Ref { read: next };
        let elem = self.read_index(refe.read)?;
        Some((refe, elem))
    }

    fn previous_of_ref(&self, reference: &Ref) -> Option<(Ref, &Self::T)> {
        let previous = reference.read.wrapping_sub(1);

        if !self.is_valid_read(previous) {
            return None;
        }

        let refe = Ref { read: previous };
        let elem = self.read_index(refe.read)?;
        Some((refe, elem))
    }

    fn is_valid_reference(&self, reference: &Ref) -> bool {
        self.is_valid_read(reference.read)
    }

    fn read_reference(&self, reference: &Ref) -> Option<&Self::T> {
        if !self.is_valid_reference(reference) {
            return None;
        }

        self.read_index(reference.read)
    }

    fn last(&self) -> Option<(Ref, &T)> {
        if self.is_empty() {
            return None;
        }

        let last_write = self.write.wrapping_sub(1);
        let refe = Ref { read: last_write };
        let elem = self.read_index(refe.read)?;
        Some((refe, elem))
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
