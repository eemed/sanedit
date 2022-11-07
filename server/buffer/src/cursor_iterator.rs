/// Cursor iterator trait. Uses a cursor that moves between elements. The
/// positions between these elements are arbitrary. For example when using `pos()`
/// and then `next()` and then `pos()`. The positions are not guaranteed to be
/// for example 1 and 2. They could be 1 and 34.
pub trait CursorIterator {
    type Item;

    /// Current cursor position
    fn pos(&self) -> usize;

    /// Current item on the cursor. Returns none if we are at the end.
    fn get(&self) -> Option<Self::Item>;

    /// Advance cursor to the next element and return it
    fn next(&mut self) -> Option<Self::Item>;

    /// Advance cursor to the previous element and return it
    fn prev(&mut self) -> Option<Self::Item>;

    /// Find next item for which `f` returns true
    fn find_next<F: Fn(&Self::Item) -> bool>(&mut self, f: F) -> Option<Self::Item> {
        while let Some(item) = self.next() {
            if f(&item) {
                return Some(item);
            }
        }

        None
    }

    fn find_next_pos<F: Fn(&Self::Item) -> bool>(&mut self, f: F) -> usize {
        self.find_next(f);
        self.pos()
    }

    /// Find previous item for which `f` returns true
    fn find_prev<F: Fn(&Self::Item) -> bool>(&mut self, f: F) -> Option<Self::Item> {
        while let Some(item) = self.prev() {
            if f(&item) {
                return Some(item);
            }
        }

        None
    }

    fn find_prev_pos<F: Fn(&Self::Item) -> bool>(&mut self, f: F) -> usize {
        self.find_prev(f);
        self.pos()
    }

    /// Advance iterator to the last element
    fn last(&mut self) -> Option<Self::Item> {
        while self.next().is_some() {}
        self.prev()
    }

    /// Utility function to advance iterator `n` times.
    fn advance(&mut self, n: isize) -> Option<Self::Item> {
        for _ in 0..n.abs() {
            if n.is_positive() {
                self.next();
            } else {
                self.prev();
            }
        }

        self.get()
    }
}
