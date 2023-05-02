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

    pub fn secondary_cursors(&self) -> &[Cursor] {
        &self.cursors[1..]
    }

    pub fn cursors(&mut self) -> &[Cursor] {
        &self.cursors
    }

    pub fn cursors_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Add a new cursor
    pub fn push(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
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
