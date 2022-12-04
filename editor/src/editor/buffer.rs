mod snapshots;
mod eol;

pub(crate) enum Change {
    Insert { pos: usize, len: usize},
    Remove { pos: usize, len: usize },
    Undo,
    Redo,
}

slotmap::new_key_type!(
    pub(crate) struct BufferId;
);

#[derive(Debug)]
pub(crate) struct Buffer {}
