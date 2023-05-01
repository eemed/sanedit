mod cursor;

pub(crate) use cursor::Cursor;

#[derive(Debug)]
pub(crate) struct Cursors {
    // Cursror at index 0 is the primary cursor
    cursors: Vec<Cursor>,
}

impl Cursors {
    pub fn primary(&self) -> &Cursor {
        &self.cursors[0]
    }

    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[0]
    }

    pub fn cursors(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Add a new cursor
    pub fn add(&mut self, cursor: Cursor) {
        todo!()
    }

    /// Remove cursor at position pos
    pub fn remove(&mut self, pos: usize) {}

    /// Remove all cursors except the primary one
    pub fn remove_secondary_cursors(&mut self) {
        todo!()
    }

    /// Merge overlapping cursors into one
    pub fn merge_overlapping(&mut self) {
        todo!()
    }
}

impl Default for Cursors {
    fn default() -> Self {
        Cursors {
            cursors: vec![Cursor::default()],
        }
    }
}
