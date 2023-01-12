#[derive(Debug)]
pub(crate) enum Change {
    Insert { pos: usize, len: usize },
    Remove { pos: usize, len: usize },
    Undo,
    Redo,
}

