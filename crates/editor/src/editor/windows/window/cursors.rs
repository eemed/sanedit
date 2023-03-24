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
}

impl Default for Cursors {
    fn default() -> Self {
        Cursors {
            cursors: vec![Cursor::default()],
        }
    }
}
