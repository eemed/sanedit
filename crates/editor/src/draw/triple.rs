use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct TripleBuffer<T: Clone + Default> {
    bufs: [Arc<T>; 3],
    active: usize,
}

impl<T: Clone + Default> Default for TripleBuffer<T> {
    fn default() -> Self {
        TripleBuffer {
            bufs: [
                Arc::new(T::default()),
                Arc::new(T::default()),
                Arc::new(T::default()),
            ],
            active: 0,
        }
    }
}

impl<T: Clone + Default> TripleBuffer<T> {
    pub fn get(&self) -> Arc<T> {
        self.bufs[self.active].clone()
    }

    pub fn next_mut(&mut self) -> &mut T {
        self.active = (self.active + 1) % self.bufs.len();
        let elem = &mut self.bufs[self.active];
        Arc::make_mut(elem)
    }
}
